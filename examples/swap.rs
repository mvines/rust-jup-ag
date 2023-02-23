use jup_ag::QuoteConfig;
use solana_sdk::transaction::VersionedTransaction;

use {
    itertools::Itertools,
    solana_client::nonblocking::rpc_client::RpcClient,
    solana_sdk::{
        commitment_config::CommitmentConfig,
        pubkey,
        signature::{read_keypair_file, Keypair, Signer},
    },
    spl_token::{amount_to_ui_amount, ui_amount_to_amount},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sol = pubkey!("So11111111111111111111111111111111111111112");
    let msol = pubkey!("mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So");

    let keypair = read_keypair_file("swap_example.json").unwrap_or_else(|err| {
        println!("------------------------------------------------------------------------------------------------");
        println!("Failed to read `swap_example.json`: {err}");
        println!();
        println!("An ephemeral keypair will be used instead. For a more realistic example, create a new keypair at");
        println!("that location and fund it with a small amount of SOL.");
        println!("------------------------------------------------------------------------------------------------");
        println!();
        Keypair::new()
    });

    let rpc_client = RpcClient::new_with_commitment(
        "https://api.metaplex.solana.com".into(),
        CommitmentConfig::confirmed(),
    );

    let msol_token_address =
        spl_associated_token_account::get_associated_token_address(&keypair.pubkey(), &msol);
    println!(
        "Pre-swap SOL balance: {}",
        amount_to_ui_amount(rpc_client.get_balance(&keypair.pubkey()).await?, 9)
    );
    println!(
        "Pre-swap mSOL balance: {}",
        amount_to_ui_amount(
            rpc_client
                .get_token_account_balance(&msol_token_address)
                .await?
                .amount
                .parse::<u64>()?,
            9
        )
    );

    let slippage_bps = 100;
    let only_direct_routes = false;
    let quotes = jup_ag::quote(
        sol,
        msol,
        ui_amount_to_amount(0.01, 9),
        QuoteConfig {
            only_direct_routes,
            slippage_bps: Some(slippage_bps),
            ..QuoteConfig::default()
        },
    )
    .await?
    .data;

    let quote = quotes.get(0).ok_or("No quotes found for SOL to mSOL")?;

    let route = quote
        .market_infos
        .iter()
        .map(|market_info| market_info.label.clone())
        .join(", ");
    println!(
        "Quote: {} SOL for {} mSOL via {} (worst case with slippage: {}). Impact: {:.2}%",
        amount_to_ui_amount(quote.in_amount, 9),
        amount_to_ui_amount(quote.out_amount, 9),
        route,
        amount_to_ui_amount(quote.other_amount_threshold, 9),
        quote.price_impact_pct * 100.
    );

    let jup_ag::Swap { swap_transaction } = jup_ag::swap(quote.clone(), keypair.pubkey()).await?;

    let swap_transaction = VersionedTransaction::try_new(swap_transaction.message, &[&keypair])?;
    println!(
        "Simulating swap transaction: {}",
        swap_transaction.signatures[0]
    );
    let response = rpc_client.simulate_transaction(&swap_transaction).await?;
    println!("  {:#?}", response.value);
    println!("Sending transaction: {}", swap_transaction.signatures[0]);
    let _ = rpc_client
        .send_and_confirm_transaction_with_spinner(&swap_transaction)
        .await?;

    println!(
        "Post-swap SOL balance: {}",
        amount_to_ui_amount(rpc_client.get_balance(&keypair.pubkey()).await?, 9)
    );
    println!(
        "Post-swap mSOL balance: {}",
        amount_to_ui_amount(
            rpc_client
                .get_token_account_balance(&msol_token_address)
                .await?
                .amount
                .parse::<u64>()?,
            9
        )
    );

    Ok(())
}
