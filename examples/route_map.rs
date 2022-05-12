#[tokio::main]
async fn main() -> jup_ag::Result<()> {
    let sol = solana_sdk::pubkey!("So11111111111111111111111111111111111111112");

    let only_direct_routes = false;
    let route_map = jup_ag::route_map(only_direct_routes).await?;

    println!("{} supported input tokens", route_map.len());

    let sol_output_tokens = route_map.get(&sol).expect("SOL is an input token");
    println!(
        "{} supported output tokens for SOL:",
        sol_output_tokens.len()
    );
    for output_token in route_map.get(&sol).expect("SOL").iter() {
        println!("- {}", output_token);
    }

    Ok(())
}
