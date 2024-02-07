use futures::stream::{FuturesUnordered, StreamExt};
use hickory_resolver::{
    config::*,
    error::ResolveError,
    name_server::{GenericConnector, TokioRuntimeProvider},
    AsyncResolver, TokioAsyncResolver,
};
use serde::Deserialize;
use tokio::time::Duration;
use tokio::time::Instant;

const MAX: usize = 1000;

type MyResult = (String, Duration);

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

pub async fn dns_resolution(url: String) -> Result<MyResult, ResolveError> {
    let resolver = TokioAsyncResolver::tokio(ResolverConfig::google(), ResolverOpts::default());
    let now = Instant::now();
    let _ = resolver.lookup_ip(url.clone()).await;
    let time = now.elapsed();
    Ok((url, time))
}

pub fn get_resolvers() -> Vec<AsyncResolver<GenericConnector<TokioRuntimeProvider>>> {
    vec![
        TokioAsyncResolver::tokio(ResolverConfig::google(), ResolverOpts::default()),
        TokioAsyncResolver::tokio(ResolverConfig::cloudflare(), ResolverOpts::default()),
        TokioAsyncResolver::tokio(ResolverConfig::quad9(), ResolverOpts::default()),
    ]
}

pub async fn fetch_and_query() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // setup
    let setup = Instant::now();
    let urls = read_csv()?; // gets ~1000 domains for testing purposes
    let resolvers = get_resolvers();
    let end_setup = setup.elapsed();
    println!("Setup completed in {:?}", end_setup);

    let resolution = Instant::now();
    let mut futures = Vec::new();
    for (index, url) in urls.into_iter().enumerate() {
        let selected_resolver_idx = index % resolvers.len();
        let resolver = resolvers[selected_resolver_idx].clone();
        let future = dns_resolution(url);
        futures.push(tokio::spawn(future));
    }
    for future in futures {
        let _ = future.await;
    }
    let end_resolution = resolution.elapsed();
    println!("Resolution completed in {:?}", end_resolution);
    Ok(())
}

pub async fn fetch_and_query_futures() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // setup
    let setup = Instant::now();
    let urls = read_csv()?; // gets ~1000 domains for testing purposes
    let end_setup = setup.elapsed();
    println!("Setup completed in {:?}", end_setup);

    let resolution = Instant::now();
    let futures = FuturesUnordered::new();
    for (index, url) in urls.into_iter().enumerate() {
        futures.push(dns_resolution(url));
    }
    let res: Vec<_> = futures.collect().await;
    println!("{:?}", res);
    let end_resolution = resolution.elapsed();
    println!("Resolution completed in {:?}", end_resolution);
    Ok(())
}
