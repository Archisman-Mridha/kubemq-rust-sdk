#![allow(non_snake_case)]

mod proto {
  // Including code generated from the .proto files.
  tonic::include_proto!("kubemq");
}

pub mod grpc {
  use anyhow::Result;
  use tonic::transport::{Channel, ClientTlsConfig, Certificate};
  use crate::{client::ClientConfig, proto::kubemq_client::KubemqClient};

  const DEFAULT_MAX_SEND_SIZE: usize= 1024 * 1024 * 100;    // 100 MB
  const DEFAULT_MAX_RECEIVE_SIZE: usize= 1024 * 1024 * 100; // 100 MB

  pub struct GrpcTransport {
    pub connection: KubemqClient<Channel>
  }

  impl GrpcTransport {
    // new establishes a gRPC connection with the KubeMQ server.
    pub async fn new(clientConfig: &ClientConfig) -> Result<GrpcTransport> {
      let uri: &'static str= Box::leak(clientConfig.uri.clone( ).into_boxed_str( ));

      let mut channel= Channel::from_static(uri);
      if let Some(pemEncodedTlsCert)= clientConfig.pemEncodedTlsCert.clone( ) {
        let tlsConfig= ClientTlsConfig::new( )
                                        .ca_certificate(Certificate::from_pem(pemEncodedTlsCert));
        channel= channel.tls_config(tlsConfig)?;
      }
      let channel= channel.connect( ).await?;

      let connection= KubemqClient::new(channel)
                                    .max_decoding_message_size(DEFAULT_MAX_RECEIVE_SIZE)
                                    .max_encoding_message_size(DEFAULT_MAX_SEND_SIZE);

      Ok(GrpcTransport { connection })
    }
  }
}

pub mod client {

  mod config {
    use std::time::Duration;

    #[derive(Clone)]
    pub struct AutoReconnectConfig {
      pub interval: Duration,
      pub maxTries: i32
    }

    pub struct ClientConfig {
      pub uri: String,
      pub pemEncodedTlsCert: Option<String>,
      pub authToken: Option<String>,
      pub clientId: String,
      pub autoReconnectConfig: Option<AutoReconnectConfig>
    }

    #[derive(Default)]
    pub struct ClientConfigBuilder {
      uri: String,
      pemEncodedTlsCert: Option<String>,
      authToken: Option<String>,
      clientId: String,
      autoReconnectConfig: Option<AutoReconnectConfig>
    }

    impl ClientConfigBuilder {
      pub fn new(uri: String, clientId: String) -> Self {
        Self {
          uri,
          clientId,
          ..Default::default( )
        }
      }

      pub fn withPemEncodedTlsCert(&mut self, pemEncodedTlsCert: String) -> &mut Self {
        self.pemEncodedTlsCert= Some(pemEncodedTlsCert);
        self
      }

      pub fn withAuthToken(&mut self, authToken: String) -> &mut Self {
        self.authToken= Some(authToken);
        self
      }

      pub fn withAutoReconnect(&mut self, autoReconnectConfig: AutoReconnectConfig) -> &mut Self {
        self.autoReconnectConfig= Some(autoReconnectConfig);
        self
      }

      pub fn build(&self) -> ClientConfig {
        ClientConfig {
          uri: self.uri.clone( ),
          pemEncodedTlsCert: self.pemEncodedTlsCert.clone( ),
          authToken: self.authToken.clone( ),
          clientId: self.clientId.clone( ),
          autoReconnectConfig: self.autoReconnectConfig.clone( )
        }
      }
    }
  }

  pub use config::*;
  use tonic::Request;
  use uuid::Uuid;
  use crate::grpc::GrpcTransport;
  use anyhow::Result;

  pub struct KubemqClient {
    pub config: ClientConfig,
    pub transport: GrpcTransport
  }

  impl KubemqClient {
    // new creates a gRPC based KubeMQ client.
    pub async fn new(config: ClientConfig) -> Result<Self> {
      let transport= GrpcTransport::new(&config).await?;

      Ok(KubemqClient {
        config: config,
        transport: transport
      })
    }

    // ping pings the KubeMQ server to check whether the underlying connection is healthy or not.
    pub async fn ping(&mut self) -> Result<( )> {
      self.transport.connection.ping(Request::new(( ))).await?;

      Ok(( ))
    }
  }

  impl KubemqClient {
    pub fn getClientId(&self) -> String {
      self.config.clientId.clone( )
    }

    pub fn parseRequestOrMessageId(&self, requestId: Option<String>) -> String {
      match requestId {
        Some(value) => value,
        None => Uuid::new_v4( ).to_string( )
      }
    }
  }
}

pub mod queues {
  use std::{time::Duration, collections::{HashMap, hash_map::RandomState}};
  use crate::{proto::{ReceiveQueueMessagesRequest, QueueMessage, QueueMessagePolicy}, client::KubemqClient};
  use anyhow::{Result, anyhow};
  use derive_more::Constructor;

  #[derive(Constructor)]
  pub struct Queues {
    client: KubemqClient
  }

  pub struct SendMessageArgs {
    messageId: Option<String>,
    channel: String,
    metadata: String,
    body: Vec<u8>,
    tags: HashMap<String, String, RandomState>,
    policy: Option<QueueMessagePolicy>
  }

  #[derive(Default)]
  pub struct SendMessageArgsBuilder {
    messageId: Option<String>,
    channel: String,
    body: Vec<u8>,
    metadata: String,
    tags: HashMap<String, String, RandomState>,
    policy: Option<QueueMessagePolicy>
  }

  impl SendMessageArgsBuilder {
    pub fn new(channel: String, body: Vec<u8>) -> Self {
      SendMessageArgsBuilder {
        channel,
        body,
        ..Default::default( )
      }
    }

    pub fn withMessageId(&mut self, messageId: String) -> &mut Self {
      self.messageId= Some(messageId);
      self
    }

    pub fn withMetadata(&mut self, metadata: String) -> &mut Self {
      self.metadata= metadata;
      self
    }

    pub fn withTags(&mut self, tags: HashMap<String, String>) -> &mut Self {
      self.tags= tags;
      self
    }

    pub fn withPolicy(&mut self, policy: QueueMessagePolicy) -> &mut Self {
      self.policy= Some(policy);
      self
    }

    pub fn build(&self) -> SendMessageArgs {
      SendMessageArgs {
        messageId: self.messageId.clone( ),
        channel: self.channel.clone( ),
        body: self.body.clone( ),
        metadata: self.metadata.clone( ),
        tags: self.tags.clone( ),
        policy: self.policy.clone( ),
      }
    }
  }

  #[derive(Debug)]
  pub struct SendMessageOutput {
    pub messageId: String,
    pub sentAt: i64,
    pub expirationAt: i64,
    pub delayedTo: i64,
    pub isError: bool,
    pub error: String
  }

  #[derive(Default)]
  pub struct PullOrPeekMessagesArgs {
    pub requestId: Option<String>,
    pub channel: String,
    pub clientId: Option<String>,
    pub maxNumberOfMessages: i32,
    pub waitTimeout: Duration
  }

  #[derive(Debug)]
  pub struct PullOrPeekMessagesOutput {
    pub requestId: String,
    pub messages: Vec<QueueMessage>,
    pub validMessagesCount: i32,
    pub expiredMessagesCount: i32,
    pub isPeek: bool
  }

  impl Queues {

    // send sends a message to a KubeMQ queue.
    pub async fn send(&mut self, args: SendMessageArgs) -> Result<SendMessageOutput> {
      let message= QueueMessage {
        message_id: self.client.parseRequestOrMessageId(args.messageId),
        channel: args.channel,
        client_id: self.client.getClientId( ),
        body: args.body,
        metadata: args.metadata,
        tags: args.tags,
        policy: args.policy,
        ..Default::default( )
      };

      let response= self.client.transport.connection.send_queue_message(message).await?.into_inner( );

      Ok(SendMessageOutput {
        messageId: response.message_id,
        sentAt: response.sent_at,
        expirationAt: response.expiration_at,
        delayedTo: response.delayed_to,
        isError: response.is_error,
        error: response.error
      })
    }

    // pull consumes messages from a queue channel, removing them from the queue.
    pub async fn pull(&mut self, args: PullOrPeekMessagesArgs) -> Result<PullOrPeekMessagesOutput> {
      self.pullOrPeek(args, false).await
    }

    // peek consumes messages from a queue channel without removing them from the queue.
    pub async fn peek(&mut self, args: PullOrPeekMessagesArgs) -> Result<PullOrPeekMessagesOutput> {
      self.pullOrPeek(args, true).await
    }

    async fn pullOrPeek(&mut self, args: PullOrPeekMessagesArgs, peek: bool) -> Result<PullOrPeekMessagesOutput> {
      let request= ReceiveQueueMessagesRequest {
        request_id: self.client.parseRequestOrMessageId(args.requestId),
        client_id: self.client.getClientId( ),
        channel: args.channel,
        max_number_of_messages: args.maxNumberOfMessages,
        is_peak: peek,
        wait_time_seconds: args.waitTimeout.as_secs( ) as i32
      };

      let response= self.client.transport.connection.receive_queue_messages(request).await?.into_inner( );

      match response.is_error {
        true => Err(anyhow!(response.error)),

        _ => Ok(PullOrPeekMessagesOutput {
          requestId: response.request_id,
          messages: response.messages,
          validMessagesCount: response.messages_received,
          expiredMessagesCount: response.messages_expired,
          isPeek: response.is_peak,
        })
      }
    }

    pub async fn stream(&self) {
      unimplemented!( )
    }

  }

}