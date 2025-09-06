use serde::{Deserialize, Serialize};




use super::{
    zoo_message::{MessageBody, MessageData, NodeApiData, ZooMessage},
    zoo_message_error::ZooMessageError,
    zoo_message_schemas::{JobMessage, MessageSchemaType},
};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum EncryptionStatus {
    NotCurrentlyEncrypted,
    BodyEncrypted,
    ContentEncrypted,
}

impl ZooMessage {
    pub fn get_message_content(&self) -> Result<String, ZooMessageError> {
        match &self.body {
            MessageBody::Unencrypted(body) => match &body.message_data {
                MessageData::Unencrypted(data) => Ok(data.message_raw_content.clone()),
                _ => Err(ZooMessageError::InvalidMessageSchemaType(
                    "Message data is encrypted".into(),
                )),
            },
            _ => Err(ZooMessageError::MissingMessageBody("Missing message body".into())),
        }
    }

    pub fn get_message_inbox(&self) -> Result<String, ZooMessageError> {
        match &self.body {
            MessageBody::Unencrypted(body) => Ok(body.internal_metadata.inbox.clone()),
            _ => Err(ZooMessageError::MissingMessageBody("Missing message body".into())),
        }
    }

    pub fn get_message_parent_key(&self) -> Result<String, ZooMessageError> {
        match &self.body {
            MessageBody::Unencrypted(body) => match &body.message_data {
                MessageData::Unencrypted(data) => {
                    if data.message_content_schema == MessageSchemaType::JobMessageSchema {
                        let job_message: JobMessage =
                            serde_json::from_str(&data.message_raw_content).map_err(|_| {
                                ZooMessageError::InvalidMessageSchemaType("Failed to parse JobMessage".into())
                            })?;
                        Ok(job_message.parent.unwrap_or_default())
                    } else {
                        Err(ZooMessageError::InvalidMessageSchemaType(
                            "Not a JobMessageSchema".into(),
                        ))
                    }
                }
                _ => Err(ZooMessageError::InvalidMessageSchemaType(
                    "Message data is encrypted".into(),
                )),
            },
            _ => Err(ZooMessageError::MissingMessageBody("Missing message body".into())),
        }
    }

    pub fn get_message_content_schema(&self) -> Result<MessageSchemaType, ZooMessageError> {
        match &self.body {
            MessageBody::Unencrypted(body) => match &body.message_data {
                MessageData::Unencrypted(data) => Ok(data.message_content_schema.clone()),
                _ => Err(ZooMessageError::InvalidMessageSchemaType(
                    "Message data is encrypted".into(),
                )),
            },
            _ => Err(ZooMessageError::MissingMessageBody("Missing message body".into())),
        }
    }

    pub fn get_sender_subidentity(&self) -> Option<String> {
        match &self.body {
            MessageBody::Unencrypted(body) => {
                if body.internal_metadata.sender_subidentity.is_empty() {
                    Some("".to_string())
                } else {
                    Some(body.internal_metadata.sender_subidentity.clone())
                }
            }
            _ => None,
        }
    }

    pub fn get_sender_intra_sender(&self) -> String {
        self.external_metadata.intra_sender.to_string()
    }

    pub fn get_recipient_subidentity(&self) -> Option<String> {
        match &self.body {
            MessageBody::Unencrypted(body) => {
                if body.internal_metadata.recipient_subidentity.is_empty() {
                    Some("".to_string())
                } else {
                    Some(body.internal_metadata.recipient_subidentity.clone())
                }
            }
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn is_body_currently_encrypted(&self) -> bool {
        matches!(self.body, MessageBody::Encrypted(_))
    }

    #[allow(dead_code)]
    pub fn is_content_currently_encrypted(&self) -> bool {
        match &self.body {
            MessageBody::Encrypted(_) => true,
            MessageBody::Unencrypted(body) => matches!(body.message_data, MessageData::Encrypted(_)),
        }
    }

    #[allow(dead_code)]
    pub fn get_encryption_status(self) -> EncryptionStatus {
        if self.is_body_currently_encrypted() {
            EncryptionStatus::BodyEncrypted
        } else if self.is_content_currently_encrypted() {
            EncryptionStatus::ContentEncrypted
        } else {
            EncryptionStatus::NotCurrentlyEncrypted
        }
    }

    /// Attempts to update the node_api_data inside of the inner metadata. Errors if the message is encrypted.
    pub fn update_node_api_data(mut self, node_api_data: Option<NodeApiData>) -> Result<Self, ZooMessageError> {
        match &mut self.body {
            MessageBody::Unencrypted(body) => {
                body.internal_metadata.node_api_data = node_api_data;
                Ok(self)
            }
            MessageBody::Encrypted(_) => Err(ZooMessageError::AlreadyEncrypted(
                "Cannot update node_api_data of encrypted message.".to_string(),
            )),
        }
    }

    pub fn encode_message(&self) -> Result<Vec<u8>, ZooMessageError> {
        serde_json::to_vec(&self).map_err(|err| ZooMessageError::SerializationError(err.to_string()))
    }

    pub fn decode_message_result(encoded: Vec<u8>) -> Result<Self, ZooMessageError> {
        // Try to deserialize as JSON first
        if let Ok(str_data) = std::str::from_utf8(&encoded) {
            if str_data.starts_with('{') && str_data.ends_with('}') {
                if let Ok(message) = serde_json::from_str::<ZooMessage>(str_data) {
                    return Ok(message);
                }
            }
        }

        // If JSON deserialization failed, return an error
        Err(ZooMessageError::DecryptionError(
            "Failed to decode message".to_string(),
        ))
    }

    pub fn to_string(&self) -> Result<String, ZooMessageError> {
        let encoded = self.encode_message()?;
        String::from_utf8(encoded).map_err(|err| ZooMessageError::SerializationError(err.to_string()))
    }

    pub fn from_string(s: String) -> Result<Self, ZooMessageError> {
        let bytes = s.into_bytes();
        Self::decode_message_result(bytes)
    }

    pub fn from_str(s: &str) -> Result<Self, ZooMessageError> {
        let bytes = s.as_bytes();
        Self::decode_message_result(bytes.to_vec())
    }

    pub fn validate_message_schema(&self, schema: MessageSchemaType) -> Result<(), ZooMessageError> {
        if let MessageBody::Unencrypted(body) = &self.body {
            if let MessageData::Unencrypted(data) = &body.message_data {
                if data.message_content_schema != schema {
                    return Err(ZooMessageError::InvalidMessageSchemaType(
                        "Invalid message schema type".into(),
                    ));
                }
            } else {
                return Err(ZooMessageError::InvalidMessageSchemaType(
                    "Message data is encrypted".into(),
                ));
            }
        } else {
            return Err(ZooMessageError::MissingMessageBody("Missing message body".into()));
        }
        Ok(())
    }

    pub fn is_receiver_subidentity_main(&self) -> bool {
        match &self.body {
            MessageBody::Unencrypted(body) => {
                body.internal_metadata.recipient_subidentity == "main"
            }
            _ => false,
        }
    }

    pub fn is_receiver_subidentity_agent(&self) -> bool {
        match &self.body {
            MessageBody::Unencrypted(body) => {
                body.internal_metadata.recipient_subidentity.contains("agent")
            }
            _ => false,
        }
    }
}
