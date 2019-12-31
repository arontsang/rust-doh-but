use crate::resolver::DnsResolver;
use arr_macro::arr;
use bytes::{BytesMut, Bytes, BufMut};
use tokio::task;
use std::net::{SocketAddr};

pub async fn start<T : DnsResolver + 'static >(bind_address: SocketAddr, resolver: async_std::sync::Arc<T>) -> () {


        let socket = tokio::net::UdpSocket::bind(bind_address).await;
        let socket = socket.unwrap();

        let (mut _receiver, mut _sender) = socket.split();

        let (reply_queue, reply_tasks) = tokio::sync::mpsc::channel::<(SocketAddr, Bytes)>(100);

        let listen_request_task = task::spawn_local(async move {
            let mut queue = reply_tasks;
            loop {
                let (client, payload) = queue.recv().await.unwrap();
                _sender.send_to(&payload, &client).await.unwrap();
            }
        });

        let resolver = async_std::sync::Arc::clone(&resolver);

        let request_dispatch_task = task::spawn_local(async move {
            let mut receiver = _receiver;
            loop {
                let mut local_reply_queue = reply_queue.clone();
                let local_resolver_copy = async_std::sync::Arc::clone(&resolver);
                let mut buffer = BytesMut::with_capacity(1500);
                {
                    let mut read_buffer = arr![0u8; 1500];
                    let (length, client) = receiver.recv_from(&mut read_buffer).await.unwrap();

                    buffer.put(&read_buffer[0..length]);

                    let query = buffer.freeze();

                    task::spawn_local(async move {
                        let reply = local_resolver_copy.resolve(query).await.unwrap();

                        local_reply_queue.send((client, reply)).await.unwrap();
                    });

                }
            }
        });

        listen_request_task.await.unwrap();
        request_dispatch_task.await.unwrap();
}

