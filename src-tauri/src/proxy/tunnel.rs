use anyhow::Result;
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use tokio::io::copy_bidirectional;
use tokio::net::TcpStream;

pub async fn tunnel(upgraded: Upgraded, target: String) -> Result<()> {
    let mut client = TokioIo::new(upgraded);
    let mut server = TcpStream::connect(&target).await?;
    let _ = copy_bidirectional(&mut client, &mut server).await;
    Ok(())
}
