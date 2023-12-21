use jup_ag::QuoteConfig;

use {
    itertools::Itertools,
    solana_sdk::pubkey,
    spl_token::{amount_to_ui_amount, ui_amount_to_amount},
};

#[tokio::main]
async fn main() -> jup_ag::Result<()> {
    let usdc = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
    let sol = pubkey!("So11111111111111111111111111111111111111112");
    let msol = pubkey!("mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So");

    let ui_amount = 1.;

    for (output_token, output_decimals) in [(usdc, 6), (msol, 9), (sol, 9)] {
        let data = jup_ag::price(sol, output_token, ui_amount).await?;
        println!(
            "Price for {} {} is {} {}",
            ui_amount, data.input_symbol, data.price, data.output_symbol
        );

        let slippage_bps = 100;
        let only_direct_routes = false;
        let quotes = jup_ag::quote(
            sol,
            output_token,
            ui_amount_to_amount(ui_amount, 9),
            QuoteConfig {
                only_direct_routes,
                slippage_bps: Some(slippage_bps),
                ..QuoteConfig::default()
            },
        )
        .await?;

        println!("Received {} quotes:", quotes.route_plan.len());
        for (i, quote) in quotes.route_plan.into_iter().enumerate() {
            let route = quote
                .swap_info
                .label
                .unwrap_or_else(||"Unknown DEX".to_string());
            println!(
                "{}. {} {} for {} {} via {} (worst case with slippage: {}). Impact: {:.2}%",
                i,
                amount_to_ui_amount(quote.swap_info.in_amount, 9),
                data.input_symbol,
                amount_to_ui_amount(quote.swap_info.out_amount, output_decimals),
                data.output_symbol,
                route,
                amount_to_ui_amount(quotes.other_amount_threshold, output_decimals),
                quotes.price_impact_pct * 100.
            );
        }
        println!();
    }

    Ok(())
}
