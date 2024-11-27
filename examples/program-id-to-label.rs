#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let result = jup_ag::program_id_to_label().await?;

    println!("Program ID's and DEXes: {:?}", result);

    Ok(())
}
