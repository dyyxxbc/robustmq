use metadata_struct::mqtt::cluster::MQTTCluster;
use protocol::mqtt::common::{
    ConnAck, ConnAckProperties, ConnectProperties, ConnectReturnCode, Disconnect,
    DisconnectProperties, DisconnectReasonCode, MQTTPacket, PingResp, PubAck, PubAckProperties,
    PubAckReason, PubComp, PubCompProperties, PubCompReason, PubRec, PubRecProperties,
    PubRecReason, PubRel, PubRelProperties, PubRelReason, SubAck, SubAckProperties,
    SubscribeReasonCode, UnsubAck, UnsubAckProperties, UnsubAckReason,
};

use super::{
    connection::{response_information, Connection},
    validator::is_request_problem_info,
};

pub fn response_packet_matt5_connect_success(
    cluster: &MQTTCluster,
    client_id: String,
    auto_client_id: bool,
    session_expiry_interval: u32,
    session_present: bool,
    connect_properties: &Option<ConnectProperties>,
) -> MQTTPacket {
    let assigned_client_identifier = if auto_client_id {
        Some(client_id)
    } else {
        None
    };

    let properties = ConnAckProperties {
        session_expiry_interval: Some(session_expiry_interval),
        receive_max: Some(cluster.receive_max()),
        max_qos: Some(cluster.max_qos().into()),
        retain_available: Some(cluster.retain_available()),
        max_packet_size: Some(cluster.max_packet_size()),
        assigned_client_identifier: assigned_client_identifier,
        topic_alias_max: Some(cluster.topic_alias_max()),
        reason_string: None,
        user_properties: Vec::new(),
        wildcard_subscription_available: Some(cluster.wildcard_subscription_available()),
        subscription_identifiers_available: Some(cluster.subscription_identifiers_available()),
        shared_subscription_available: Some(cluster.shared_subscription_available()),
        server_keep_alive: Some(cluster.server_keep_alive()),
        response_information: response_information(connect_properties),
        server_reference: None,
        authentication_method: None,
        authentication_data: None,
    };
    return MQTTPacket::ConnAck(
        ConnAck {
            session_present,
            code: ConnectReturnCode::Success,
        },
        Some(properties),
    );
}

pub fn response_packet_matt5_connect_fail(
    code: ConnectReturnCode,
    connect_properties: &Option<ConnectProperties>,
    error: Option<String>,
) -> MQTTPacket {
    let mut ack_properties = ConnAckProperties::default();
    if is_request_problem_info(connect_properties) {
        ack_properties.reason_string = error;
    }
    return MQTTPacket::ConnAck(
        ConnAck {
            session_present: false,
            code,
        },
        Some(ack_properties),
    );
}

pub fn response_packet_matt5_connect_fail_by_code(code: ConnectReturnCode) -> MQTTPacket {
    return MQTTPacket::ConnAck(
        ConnAck {
            session_present: false,
            code,
        },
        None,
    );
}

pub fn response_packet_matt5_distinct(
    code: DisconnectReasonCode,
    connection: &Connection,
    reason_string: Option<String>,
) -> MQTTPacket {
    let mut properteis = DisconnectProperties::default();
    if connection.is_response_proplem_info() {
        properteis.reason_string = reason_string;
    }

    return MQTTPacket::Disconnect(Disconnect { reason_code: code }, None);
}

pub fn response_packet_matt_distinct(
    code: DisconnectReasonCode,
    reason_string: Option<String>,
) -> MQTTPacket {
    let mut properteis = DisconnectProperties::default();
    properteis.reason_string = reason_string;
    return MQTTPacket::Disconnect(Disconnect { reason_code: code }, None);
}

pub fn response_packet_matt5_puback_success(
    reason: PubAckReason,
    pkid: u16,
    user_properties: Vec<(String, String)>,
) -> MQTTPacket {
    let pub_ack = PubAck { pkid, reason };
    let properties = Some(PubAckProperties {
        reason_string: None,
        user_properties: user_properties,
    });
    return MQTTPacket::PubAck(pub_ack, properties);
}

pub fn response_packet_matt5_puback_fail(
    connection: &Connection,
    pkid: u16,
    reason: PubAckReason,
    reason_string: Option<String>,
) -> MQTTPacket {
    let pub_ack = PubAck { pkid: 0, reason };
    let mut properties = PubAckProperties::default();
    if connection.is_response_proplem_info() {
        properties.reason_string = reason_string;
    }
    return MQTTPacket::PubAck(pub_ack, Some(properties));
}

pub fn response_packet_matt5_pubrec_success(
    reason: PubRecReason,
    pkid: u16,
    user_properties: Vec<(String, String)>,
) -> MQTTPacket {
    let rec = PubRec { pkid, reason };
    let properties = Some(PubRecProperties {
        reason_string: None,
        user_properties: user_properties,
    });
    return MQTTPacket::PubRec(rec, properties);
}

pub fn response_packet_matt5_pubrec_fail(
    connection: &Connection,
    pkid: u16,
    reason: PubRecReason,
    reason_string: Option<String>,
) -> MQTTPacket {
    let pub_ack = PubRec { pkid, reason };
    let mut properties = PubRecProperties::default();
    if connection.is_response_proplem_info() {
        properties.reason_string = reason_string;
    }
    return MQTTPacket::PubRec(pub_ack, Some(properties));
}

pub fn response_packet_matt5_pubrel_success(pkid: u16, reason: PubRelReason) -> MQTTPacket {
    let rel = PubRel { pkid, reason };
    let properties = Some(PubRelProperties::default());
    return MQTTPacket::PubRel(rel, properties);
}

pub fn response_packet_matt5_pubcomp_success(pkid: u16) -> MQTTPacket {
    let rec = PubComp {
        pkid,
        reason: PubCompReason::Success,
    };
    let properties = Some(PubCompProperties::default());
    return MQTTPacket::PubComp(rec, properties);
}

pub fn response_packet_matt5_pubcomp_fail(
    connection: &Connection,
    pkid: u16,
    reason: PubCompReason,
    reason_string: Option<String>,
) -> MQTTPacket {
    let pub_ack = PubComp { pkid, reason };
    let mut properties = PubCompProperties::default();
    if connection.is_response_proplem_info() {
        properties.reason_string = reason_string;
    }
    return MQTTPacket::PubComp(pub_ack, Some(properties));
}

pub fn response_packet_matt5_suback(
    connection: &Connection,
    pkid: u16,
    return_codes: Vec<SubscribeReasonCode>,
    reason_string: Option<String>,
) -> MQTTPacket {
    let sub_ack = SubAck { pkid, return_codes };
    let mut properties = SubAckProperties::default();
    if connection.is_response_proplem_info() {
        properties.reason_string = reason_string;
    }
    return MQTTPacket::SubAck(sub_ack, Some(properties));
}

pub fn response_packet_ping_resp() -> MQTTPacket {
    return MQTTPacket::PingResp(PingResp {});
}

pub fn response_packet_matt5_unsuback(
    connection: &Connection,
    pkid: u16,
    reasons: Vec<UnsubAckReason>,
    reason_string: Option<String>,
) -> MQTTPacket {
    let unsub_ack = UnsubAck { pkid, reasons };
    let mut properties = UnsubAckProperties::default();
    if connection.is_response_proplem_info() {
        properties.reason_string = reason_string;
    }
    return MQTTPacket::UnsubAck(unsub_ack, None);
}