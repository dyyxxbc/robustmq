use super::{cluster::Cluster, session::Session, subscriber::Subscriber, topic::Topic, user::User};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[derive(Clone, Serialize, Deserialize)]
pub enum MetadataCacheAction {
    Set,
    Del,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum MetadataCacheType {
    Cluster,
    User,
    Topic,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MetadataChangeData {
    pub action: MetadataCacheAction,
    pub data_type: MetadataCacheType,
    pub value: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MetadataCache {
    pub cluster_info: Cluster,
    pub user_info: HashMap<String, User>,
    pub session_info: HashMap<String, Session>,
    pub topic_info: HashMap<String, Topic>,
    pub subscriber_info: HashMap<String, Subscriber>,
    pub connect_id_info: HashMap<u64, String>,
    pub login_info: HashMap<u64, bool>,
}

impl MetadataCache {
    pub fn new() -> Self {
        return MetadataCache {
            user_info: HashMap::new(),
            session_info: HashMap::new(),
            cluster_info: Cluster::default(),
            topic_info: HashMap::new(),
            subscriber_info: HashMap::new(),
            connect_id_info: HashMap::new(),
            login_info: HashMap::new(),
        };
    }

    pub fn apply(&mut self, data: String) {
        let data: MetadataChangeData = serde_json::from_str(&data).unwrap();
        match data.data_type {
            MetadataCacheType::User => match data.action {
                MetadataCacheAction::Set => self.set_user(data.value),
                MetadataCacheAction::Del => self.del_user(data.value),
            },
            MetadataCacheType::Topic => match data.action {
                MetadataCacheAction::Set => {}
                MetadataCacheAction::Del => {}
            },
            MetadataCacheType::Cluster => match data.action {
                MetadataCacheAction::Set => {}
                MetadataCacheAction::Del => {}
            },
        }
    }

    pub fn set_user(&mut self, value: String) {
        let data: User = serde_json::from_str(&value).unwrap();
        self.user_info.insert(data.username.clone(), data);
    }

    pub fn del_user(&mut self, value: String) {
        let data: User = serde_json::from_str(&value).unwrap();
        self.user_info.remove(&data.username);
    }

    pub fn set_session(&mut self, client_id: String, session: Session) {
        self.session_info.insert(client_id, session);
    }

    pub fn set_client_id(&mut self, connect_id: u64, client_id: String) {
        self.connect_id_info.insert(connect_id, client_id);
    }

    pub fn set_topic(&mut self, topic_name: &String, topic: &Topic) {
        self.topic_info.insert(topic_name.clone(), topic.clone());
    }

    pub fn login_success(&mut self, connect_id: u64) {
        self.login_info.insert(connect_id, true);
    }

    pub fn is_login(&self, connect_id: u64) -> bool {
        return self.login_info.contains_key(&connect_id);
    }

    pub fn topic_exists(&self, topic: &String) -> bool {
        return self.topic_info.contains_key(topic);
    }

    pub fn remove_connect_id(&mut self, connect_id: u64) {
        if let Some(client_id) = self.connect_id_info.get(&connect_id) {
            self.session_info.remove(client_id);
            self.login_info.remove(&connect_id);
            self.connect_id_info.remove(&connect_id);
        }
    }
}