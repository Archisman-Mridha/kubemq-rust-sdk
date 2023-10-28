#![allow(non_snake_case, unused)]

use std::time::Duration;

use kubemq_rust_sdk::{client::{KubemqClient, ClientConfigBuilder}, queues::{Queues, PullOrPeekMessagesArgs, SendMessageArgsBuilder}};
use uuid::Uuid;

#[tokio::main]
async fn main( ) {
  let mut kubemqClient= KubemqClient::new(
    ClientConfigBuilder::new("http://[::1]:50000".to_string( ), Uuid::new_v4( ).to_string( ))
                         .build( )
  ).await.expect("Error creating KubeMQ client");

  kubemqClient.ping( ).await.expect("Error creating KubeMQ client");

  println!("Connected to KubeMQ");

  let mut queues= Queues::new(kubemqClient);

  let defaultChannel= "queues.default";

  queues.send(
    SendMessageArgsBuilder::new(defaultChannel.to_string( ), "Hello".as_bytes( ).to_vec( ))
                            .build( )
  ).await.expect("Error sending message to KubeMQ");
  println!("Successfully sent message to KubeMQ");

  let output= queues.peek(PullOrPeekMessagesArgs {
    channel: defaultChannel.to_string( ),

    maxNumberOfMessages: 1,
    waitTimeout: Duration::from_secs(3),

    ..Default::default( )

  }).await.expect("Error peeking messages from KubeMQ");
  println!("Successfully received message from KubeMQ");
  println!("{:?}", output);
}