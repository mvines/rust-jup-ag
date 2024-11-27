#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let result = jup_ag::tokens().await?;

    println!("Tadeable tokens: {:?}", result);

    Ok(())
}
