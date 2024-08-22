use common::{broker_addr, connect_server34, connect_server5, distinct_conn};
use common_base::tools::unique_id;
use paho_mqtt::{Message, MessageBuilder, Properties, PropertyCode};

mod common;

#[cfg(test)]
mod tests {

    use paho_mqtt::{QOS_0, QOS_1, QOS_2};

    use crate::{publish34_qos, publish5_qos};

    #[tokio::test]
    async fn client34_publish_test() {
        let num = 10;
        publish34_qos(num, QOS_0).await;
        publish34_qos(num, QOS_1).await;
        publish34_qos(num, QOS_2).await;
    }

    #[tokio::test]
    async fn client5_publish_test() {
        let num = 1;
        publish5_qos(num, QOS_0, false).await;
        publish5_qos(num, QOS_1, false).await;
        publish5_qos(num, QOS_2, false).await;

        publish5_qos(num, QOS_0, true).await;
        publish5_qos(num, QOS_1, true).await;
        publish5_qos(num, QOS_2, true).await;
    }
}
async fn publish34_qos(num: i32, qos: i32) {
    let mqtt_version = 3;
    let client_id = unique_id();
    let addr = broker_addr();
    let cli = connect_server34(mqtt_version, &client_id, &addr);
    let topic = "/tests/t1".to_string();
    for i in 0..num {
        let msg = Message::new(topic.clone(), format!("mqtt {i} message"), qos);
        match cli.publish(msg) {
            Ok(_) => {}
            Err(e) => {
                println!("{}", e);
                assert!(false);
            }
        }
    }
    distinct_conn(cli);

    let mqtt_version = 4;
    let client_id = unique_id();
    let addr = broker_addr();
    let cli = connect_server34(mqtt_version, &client_id, &addr);
    let topic = "/tests/t1".to_string();
    for i in 0..num {
        let msg = Message::new(topic.clone(), format!("mqtt {i} message"), qos);
        match cli.publish(msg) {
            Ok(_) => {}
            Err(e) => {
                println!("{}", e);
                assert!(false);
            }
        }
    }
    distinct_conn(cli);
}

async fn publish5_qos(num: i32, qos: i32, retained: bool) {
    let client_id = unique_id();
    let addr = broker_addr();
    let cli = connect_server5(&client_id, &addr);
    let topic = "/tests/t1".to_string();

    let mut props = Properties::new();
    props
        .push_u32(PropertyCode::MessageExpiryInterval, 50)
        .unwrap();
    for i in 0..num {
        let payload = format!("mqtt {i} message");
        let msg = MessageBuilder::new()
            .properties(props.clone())
            .payload(payload)
            .topic(topic.clone())
            .qos(qos)
            .retained(retained)
            .finalize();
        match cli.publish(msg) {
            Ok(_) => {}
            Err(e) => {
                println!("{}", e);
                assert!(false);
            }
        }
    }
    distinct_conn(cli);
}