use futures::stream::{FuturesUnordered, StreamExt};
use serde::Deserialize;
use tokio::time::Duration;
use tokio::time::Instant;

use hickory_client::client::{AsyncClient, ClientHandle};
use hickory_client::proto::iocompat::AsyncIoTokioAsStd;
use hickory_client::rr::{DNSClass, Name, RecordType};
use hickory_client::tcp::TcpClientStream;
use hickory_client::udp::UdpClientStream;
use std::str::FromStr;
use tokio::net::TcpStream as TokioTcpStream;

use tokio::net::UdpSocket;

const MAX: usize = 1000;

type MyResult = (String, Duration, Duration);

#[derive(Debug, Deserialize)]
struct Record {
    _index: String,
    url: String,
    _score: String,
}

pub fn read_csv() -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_path("./websites.csv")?;

    let mut urls: Vec<String> = Vec::new();
    let mut counter = 0;
    for result in rdr.deserialize() {
        if counter < MAX {
            let record: Record = result?;
            urls.push(record.url);
            counter = counter + 1;
        }
    }
    Ok(urls)
}

async fn make_client(ipport: &str, datagram: bool) -> AsyncClient {
    // Each client spawns a background task (bg) that the client synchronizes with, which still
    // seems to be a source of synchronization delays, which can be unpredictable.
    // Giving each query its own dedicated client reduces synchronization quite a bit, but there is
    // no reason an optimized client needs to synchronize between tasks at all.

    if datagram {
        // UDP client
        let stream = UdpClientStream::<UdpSocket>::new(ipport.parse().unwrap());
        let (client, bg) = AsyncClient::connect(stream).await.unwrap();
        tokio::spawn(bg);
        client
    } else {
        // TCP client
        let (stream, sender) =
            TcpClientStream::<AsyncIoTokioAsStd<TokioTcpStream>>::new(ipport.parse().unwrap());
        let (client, bg) = AsyncClient::new(stream, sender, None).await.unwrap();
        tokio::spawn(bg);
        client
    }
}

pub async fn dns_resolution(clients: Vec<AsyncClient>, url: String) -> MyResult {
    let fn_start = Instant::now();

    // If the clients Vec contains multiple clients, then we send the query to all of them. We use
    // the one that returns the fastest.
    let mut queries: FuturesUnordered<_> = clients
        .into_iter()
        .map(|mut client| client.query(Name::from_str(&url).unwrap(), DNSClass::IN, RecordType::A))
        .collect();

    let network_start = Instant::now();
    // This await is the only place where network traffic occurs. There is still quite a bit of
    // synchronization overhead here, unfortunately.
    let winning_response = queries.next().await;
    let network_duration = network_start.elapsed();

    let response = winning_response.unwrap().unwrap();
    let answers = response.answers();

    // Formatting a string is relatively CPU intensive. I'm doing it here as a stand-in for other
    // processing of the answers. In the end, this call is very insignificant compared to the
    // synchronization delays and even the network latency.
    let _printable_result = format!("{answers:?}");

    let fn_duration = fn_start.elapsed();

    (url, fn_duration, network_duration)
}

pub async fn fetch_and_query_futures() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // setup
    let setup = Instant::now();
    let mut urls = read_csv()?; // gets ~1000 domains for testing purposes

    // Use less urls if desired:
    urls.truncate(1000);

    let futures = FuturesUnordered::new();
    for (_, url) in urls.into_iter().enumerate() {
        futures.push(dns_resolution(
            vec![
                // Comment or uncomment whichever resolvers you want to connect with.
//                make_client("8.8.8.8:53", true).await,
//                make_client("1.1.1.1:53", true).await,
//                make_client("9.9.9.9:53", true).await,

//                make_client("8.8.4.4:53", true).await,
//                make_client("1.0.0.1:53", true).await,
//                make_client("149.112.112.112:53", true).await,

//                make_client("[2001:4860:4860::8888]:53", true).await,
//                make_client("[2606:4700:4700::1111]:53", true).await,
                make_client("[2620:fe::fe]:53", true).await,
//                make_client("[2001:4860:4860::8844]:53", true).await,
//                make_client("[2606:4700:4700::1001]:53", true).await,
//                make_client("[2620:fe::9]:53", true).await,
            ],
            url,
        ));
    }

    let end_setup = setup.elapsed();
    println!("Setup completed in {:?}", end_setup);

    let resolution = Instant::now();
    let res: Vec<_> = futures.collect().await;
    let end_resolution = resolution.elapsed();

    println!("{:?}", res);
    println!("Resolution completed in {:?}", end_resolution);
    Ok(())
}
