extern crate dotenv;

use dotenv::dotenv;
use std::process;
use std::env;
use log::info;
use lapin::{options::*, types::FieldTable, Connection, ConnectionProperties, Consumer};

use futures_lite::stream::StreamExt;

fn main() {
    // load .env if exists
    dotenv().ok();

    // setup some log levels
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    // "fake" queue name
    let queue_name_for_consuming = String::from("example_queue");

    let addr = env::var("AMQP_ADDR")
        .unwrap_or_else(
            |_| "amqp://127.0.0.1:5672/%2f".into()
        );

    // lock main thread with closure
    async_global_executor::block_on(async {
        // do some connecting to queue
        let consumer = connect_to_queue(&addr, &queue_name_for_consuming).await;

        // loop for receiving messages, process them by closure.
        consume(consumer, |possible_json| {
            println!("{}", possible_json)
        }).await;
    });
}

async fn connect_to_queue(addr: &String, queue_name_for_consuming: &String) -> Consumer {
    let _conn = Connection::connect(
        &addr,
        ConnectionProperties::default().with_default_executor(8),
    )
        .await
        .unwrap_or_else(|_| {
            process::exit(1);
        });

    let _channel = _conn.create_channel()
        .await
        .unwrap_or_else(|_| {
            process::exit(1);
        });

    let _queue = _channel
        .queue_declare(
            &queue_name_for_consuming,
            QueueDeclareOptions::default(),
            FieldTable::default(),
        )
        .await
        .unwrap_or_else(|_| {
            process::exit(1);
        });

    let mut _consumer = _channel
        .basic_consume(
            &queue_name_for_consuming,
            "my_consumer",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await
        .unwrap_or_else(|_| {
            process::exit(1);
        });

    _consumer
}

async fn consume<F>(mut consumer: Consumer, handle_json_string: F)
    where
        F: Fn(String)
{
    while let Ok(delivery) = consumer
        .next()
        .await
        .unwrap_or_else(|| {
            process::exit(1);
        })
    {
        let (_, delivery) = delivery;
        delivery
            .ack(BasicAckOptions::default())
            .await
            .expect("basic_ack")
        ;
        let message_body = delivery.data;
        let message_body_string = String::from_utf8(message_body);

        info!("Consumed: {:?}", message_body_string);
        match message_body_string {
            Ok(possible_json) => handle_json_string(possible_json),
            _ => {}
        }
    }
}