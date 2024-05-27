use crate::{
    core::metadata_cache::MetadataCacheManager,
    metadata::message::Message,
    qos::ack_manager::{AckManager, AckPackageType, AckPacketInfo},
    server::{tcp::packet::ResponsePackage, MQTTProtocol},
    storage::message::MessageStorage,
};
use bytes::Bytes;
use common_base::{
    log::{error, info, warn},
    tools::now_second,
};
use dashmap::DashMap;
use protocol::mqtt::{MQTTPacket, PubRel, Publish, PublishProperties, QoS};
use std::{sync::Arc, time::Duration};
use storage_adapter::storage::StorageAdapter;
use tokio::{
    sync::broadcast::{self, Sender},
    time::sleep,
};

use super::{
    sub_manager::SubscribeManager,
    subscribe::{min_qos, publish_message_to_client, publish_to_response_queue, wait_packet_ack},
};

pub struct SubscribeExclusive<S> {
    metadata_cache: Arc<MetadataCacheManager>,
    response_queue_sx4: Sender<ResponsePackage>,
    response_queue_sx5: Sender<ResponsePackage>,
    subscribe_manager: Arc<SubscribeManager>,
    message_storage: Arc<S>,
    ack_manager: Arc<AckManager>,
    // (client_id_topic_id, Sender<bool>)
    push_thread: DashMap<String, Sender<bool>>,
}

impl<S> SubscribeExclusive<S>
where
    S: StorageAdapter + Sync + Send + 'static + Clone,
{
    pub fn new(
        message_storage: Arc<S>,
        metadata_cache: Arc<MetadataCacheManager>,
        response_queue_sx4: Sender<ResponsePackage>,
        response_queue_sx5: Sender<ResponsePackage>,
        subscribe_manager: Arc<SubscribeManager>,
        ack_manager: Arc<AckManager>,
    ) -> Self {
        return SubscribeExclusive {
            message_storage,
            metadata_cache,
            response_queue_sx4,
            response_queue_sx5,
            push_thread: DashMap::with_capacity(256),
            subscribe_manager,
            ack_manager,
        };
    }

    pub async fn start(&self) {
        loop {
            self.exclusive_sub_push_thread().await;
            sleep(Duration::from_secs(1)).await;
        }
    }

    // Handles exclusive subscription push tasks
    // Exclusively subscribed messages are pushed directly to the consuming client
    async fn exclusive_sub_push_thread(&self) {
        for (_, sub_list) in self.subscribe_manager.exclusive_subscribe.clone() {
            for subscribe in sub_list {
                let client_id = subscribe.client_id.clone();
                let thread_key = format!("{}_{}", client_id, subscribe.topic_id);

                if self.push_thread.contains_key(&thread_key) {
                    continue;
                }

                let (stop_sx, mut stop_rx) = broadcast::channel(2);
                let response_queue_sx4 = self.response_queue_sx4.clone();
                let response_queue_sx5 = self.response_queue_sx5.clone();
                let metadata_cache = self.metadata_cache.clone();
                let message_storage = self.message_storage.clone();
                let ack_manager = self.ack_manager.clone();

                // Subscribe to the data push thread
                self.push_thread.insert(thread_key, stop_sx);

                tokio::spawn(async move {
                    info(format!(
                        "Exclusive push thread for client_id [{}],topic_id [{}] was started successfully",
                        client_id, subscribe.topic_id
                    ));
                    let message_storage = MessageStorage::new(message_storage);
                    let group_id = format!("system_sub_{}_{}", client_id, subscribe.topic_id);
                    let record_num = 5;
                    let max_wait_ms = 100;

                    let mut sub_ids = Vec::new();
                    if let Some(id) = subscribe.subscription_identifier {
                        sub_ids.push(id);
                    }

                    loop {
                        match stop_rx.try_recv() {
                            Ok(flag) => {
                                if flag {
                                    info(format!(
                                        "Exclusive Push thread for client_id [{}],topic_id [{}] was stopped successfully",
                                        client_id.clone(),
                                    subscribe.topic_id
                                    ));
                                    break;
                                }
                            }
                            Err(_) => {}
                        }

                        match message_storage
                            .read_topic_message(
                                subscribe.topic_id.clone(),
                                group_id.clone(),
                                record_num,
                            )
                            .await
                        {
                            Ok(result) => {
                                if result.len() == 0 {
                                    sleep(Duration::from_millis(max_wait_ms)).await;
                                    continue;
                                }

                                for record in result.clone() {
                                    let msg = match Message::decode_record(record.clone()) {
                                        Ok(msg) => msg,
                                        Err(e) => {
                                            error(format!("Storage layer message Decord failed with error message :{}",e.to_string()));
                                            continue;
                                        }
                                    };

                                    if subscribe.nolocal && (subscribe.client_id == msg.client_id) {
                                        continue;
                                    }

                                    let qos = min_qos(msg.qos, subscribe.qos);

                                    let retain = if subscribe.preserve_retain {
                                        msg.retain
                                    } else {
                                        false
                                    };

                                    let pkid: u16 =
                                        metadata_cache.get_pkid(client_id.clone()).await;

                                    let publish = Publish {
                                        dup: false,
                                        qos,
                                        pkid,
                                        retain,
                                        topic: Bytes::from(subscribe.topic_name.clone()),
                                        payload: Bytes::from(msg.payload),
                                    };

                                    let properties = PublishProperties {
                                        payload_format_indicator: None,
                                        message_expiry_interval: None,
                                        topic_alias: None,
                                        response_topic: None,
                                        correlation_data: None,
                                        user_properties: Vec::new(),
                                        subscription_identifiers: sub_ids.clone(),
                                        content_type: None,
                                    };

                                    let connect_id = if let Some(id) =
                                        metadata_cache.get_connect_id(subscribe.client_id.clone())
                                    {
                                        id
                                    } else {
                                        continue;
                                    };

                                    let resp = ResponsePackage {
                                        connection_id: connect_id,
                                        packet: MQTTPacket::Publish(publish, Some(properties)),
                                    };

                                    let mut retry_times = 1;

                                    loop {
                                        match publish_message_to_client(
                                            connect_id,
                                            client_id.clone(),
                                            pkid,
                                            ack_manager.clone(),
                                            qos,
                                            subscribe.protocol.clone(),
                                            resp.clone(),
                                            response_queue_sx4.clone(),
                                            response_queue_sx5.clone(),
                                        )
                                        .await
                                        {
                                            Ok(()) => {
                                                metadata_cache
                                                    .remove_pkid_info(client_id.clone(), pkid);

                                                match message_storage
                                                    .commit_group_offset(
                                                        subscribe.topic_id.clone(),
                                                        group_id.clone(),
                                                        record.offset,
                                                    )
                                                    .await
                                                {
                                                    Ok(_) => {}
                                                    Err(_) => {
                                                        //Occasional commit offset failures are allowed because subsequent commit offsets overwrite previous ones.
                                                        continue;
                                                    }
                                                }
                                                break;
                                            }
                                            Err(e) => {
                                                retry_times = retry_times + 1;
                                                if retry_times > 3 {
                                                    error(format!("Failed to push subscription message to client, failure message: {},topic:{},group{}",e.to_string(),
                                                    subscribe.topic_id.clone(),
                                                    group_id.clone()));
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error(format!(
                                    "Failed to read message from storage, failure message: {},topic:{},group{}",
                                    e.to_string(),
                                    subscribe.topic_id.clone(),
                                    group_id.clone()
                                ));
                                sleep(Duration::from_millis(max_wait_ms)).await;
                            }
                        }
                    }
                });
            }
        }
    }
}

// When the subscription QOS is 0, 
// the message can be pushed directly to the request return queue without the need for a retry mechanism.
pub async fn publish_message_qos0(
    protocol: MQTTProtocol,
    resp: ResponsePackage,
    response_queue_sx4: Sender<ResponsePackage>,
    response_queue_sx5: Sender<ResponsePackage>,
) {
    match publish_to_response_queue(
        protocol.clone(),
        resp.clone(),
        response_queue_sx4.clone(),
        response_queue_sx5.clone(),
    )
    .await
    {
        Ok(_) => {}
        Err(e) => {
            error(format!(
                "Failed to write QOS0 Publish message to response queue, failure message: {}",
                e.to_string()
            ));
        }
    }
}

// When the subscribed QOS is 1, we need to keep retrying to send the message to the client.
// To avoid messages that are not successfully pushed to the client. When the client Session expires,
// the push thread will exit automatically and will not attempt to push again.
pub async fn publish_message_qos1(
    client_id: String,
    pkid: u16,
    ack_manager: Arc<AckManager>,
    protocol: MQTTProtocol,
    resp: ResponsePackage,
    response_queue_sx4: Sender<ResponsePackage>,
    response_queue_sx5: Sender<ResponsePackage>,
    stop_sx: broadcast::Sender<bool>,
) {
    loop {
        match stop_sx.subscribe().try_recv() {
            Ok(flag) => {
                if flag {
                    break;
                }
            }
            Err(_) => {}
        }

        match publish_to_response_queue(
            protocol.clone(),
            resp.clone(),
            response_queue_sx4.clone(),
            response_queue_sx5.clone(),
        )
        .await
        {
            Ok(_) => {
                let (wait_puback_sx, _) = broadcast::channel(1);
                ack_manager.add(
                    client_id.clone(),
                    pkid,
                    AckPacketInfo {
                        sx: wait_puback_sx.clone(),
                        create_time: now_second(),
                    },
                );
                
                if let Some(data) = wait_packet_ack(wait_puback_sx.clone()).await {
                    if data.ack_type == AckPackageType::PubAck && data.pkid == pkid {
                        ack_manager.remove(client_id, pkid);
                        break;
                    }
                    warn(format!("Wait to receive a Publish Ack packet, but the received packet is {:?} not PubAck.",data.ack_type));
                }
            }
            Err(e) => {
                error(format!(
                    "Failed to write QOS1 Publish message to response queue, failure message: {}",
                    e.to_string()
                ));
                sleep(Duration::from_secs(10)).await;
            }
        }
    }
}

pub async fn publish_message_qos2(
    connect_id: u64,
    client_id: String,
    pkid: u16,
    ack_manager: Arc<AckManager>,
    protocol: MQTTProtocol,
    resp: ResponsePackage,
    response_queue_sx4: Sender<ResponsePackage>,
    response_queue_sx5: Sender<ResponsePackage>,
) {
    match publish_to_response_queue(
        protocol.clone(),
        resp.clone(),
        response_queue_sx4.clone(),
        response_queue_sx5.clone(),
    )
    .await
    {
        Ok(_) => {
            let (wait_pubrec_sx, _) = broadcast::channel(1);
            ack_manager.add(
                client_id.clone(),
                pkid,
                AckPacketInfo {
                    sx: wait_pubrec_sx.clone(),
                    create_time: now_second(),
                },
            );

            // wait pub rec
            loop {
                if let Some(data) = wait_packet_ack(wait_pubrec_sx.clone()).await {
                    if data.ack_type == AckPackageType::PubRec {
                        let pubrel = PubRel {
                            pkid,
                            reason: protocol::mqtt::PubRelReason::Success,
                        };

                        let pubrel_resp = ResponsePackage {
                            connection_id: connect_id,
                            packet: MQTTPacket::PubRel(pubrel, None),
                        };

                        match publish_to_response_queue(
                            protocol.clone(),
                            pubrel_resp.clone(),
                            response_queue_sx4.clone(),
                            response_queue_sx5.clone(),
                        )
                        .await
                        {
                            Ok(_) => {
                                // wait pub comp
                                let (wait_pubcomp_sx, _) = broadcast::channel(1);
                                loop {
                                    if let Some(data) =
                                        wait_packet_ack(wait_pubcomp_sx.clone()).await
                                    {
                                        if data.ack_type == AckPackageType::PubComp {
                                            ack_manager.remove(client_id.clone(), pkid);
                                            return;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error(format!(
                                            "Failed to write PubRel message to response queue, failure message: {}",
                                            e.to_string()
                                        ));
                            }
                        }
                    }
                    warn(format!("Wait to receive a Publish Rec packet, but the received packet is {:?} not PubRec.",data.ack_type));
                }
            }
        }
        Err(e) => {
            error(format!(
                "Failed to write QOS1 Publish message to response queue, failure message: {}",
                e.to_string()
            ));
        }
    }
}
