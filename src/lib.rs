/// Unofficial API Binding for Jupiter: https://jup.ag
use {
    serde::{Deserialize, Serialize},
    solana_sdk::{
        pubkey::{ParsePubkeyError, Pubkey},
        transaction::Transaction,
    },
    std::collections::HashMap,
};

mod field_as_string;

/// A `Result` alias where the `Err` case is `jup_ag::Error`.
pub type Result<T> = std::result::Result<T, Error>;

/// The Errors that may occur while using this crate
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("reqwest: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("invalid pubkey in response data: {0}")]
    ParsePubkey(#[from] ParsePubkeyError),

    #[error("base64: {0}")]
    Base64Decode(#[from] base64::DecodeError),

    #[error("bincode: {0}")]
    Bincode(#[from] bincode::Error),
}

/// Generic response with timing information
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response<T> {
    pub data: T,
    pub time_taken: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Price {
    #[serde(with = "field_as_string")]
    pub input_mint: Pubkey,
    pub input_symbol: String,
    #[serde(with = "field_as_string")]
    pub output_mint: Pubkey,
    pub output_symbol: String,
    pub amount: u64,
    pub price: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Quote {
    pub in_amount: u64,
    pub out_amount: u64,
    pub out_amount_with_slippage: u64,
    pub price_impact_pct: f64,
    pub market_infos: Vec<MarketInfo>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketInfo {
    pub id: String,
    pub label: String,
    #[serde(with = "field_as_string")]
    pub input_mint: Pubkey,
    #[serde(with = "field_as_string")]
    pub output_mint: Pubkey,
    pub not_enough_liquidity: bool,
    pub in_amount: u64,
    pub out_amount: u64,
    pub price_impact_pct: f64,
    pub lp_fee: FeeInfo,
    pub platform_fee: FeeInfo,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeeInfo {
    pub amount: u64,
    #[serde(with = "field_as_string")]
    pub mint: Pubkey,
    pub pct: f64,
}

/// Partially signed transactions required to execute a swap
#[derive(Clone, Debug)]
pub struct Swap {
    pub setup: Option<Transaction>,
    pub swap: Transaction,
    pub cleanup: Option<Transaction>,
}

/// Hashmap of possible swap routes from input mint to an array of output mints
pub type RouteMap = HashMap<Pubkey, Vec<Pubkey>>;

/// Get simple price for a given input mint, output mint and amount
pub async fn price(
    input_mint: Pubkey,
    output_mint: Pubkey,
    amount: f64,
) -> Result<Response<Price>> {
    let url = format!(
        "https://quote-api.jup.ag/v1/price?inputMint={}&outputMint={}&amount={}",
        input_mint, output_mint, amount,
    );
    reqwest::get(url)
        .await?
        .json()
        .await
        .map_err(|err| err.into())
}

/// Get quote for a given input mint, output mint and amount
pub async fn quote(
    input_mint: Pubkey,
    output_mint: Pubkey,
    amount: u64,
    only_direct_routes: bool,
    slippage: Option<f64>,
    fees_bps: Option<f64>,
) -> Result<Response<Vec<Quote>>> {
    let url = format!(
        "https://quote-api.jup.ag/v1/quote?inputMint={}&outputMint={}&amount={}&onlyDirectRoutes={}&{}{}",
        input_mint,
        output_mint,
        amount,
        only_direct_routes,
        slippage
            .map(|slippage| format!("&slippage={}", slippage))
            .unwrap_or_default(),
        fees_bps
            .map(|fees_bps| format!("&feesBps={}", fees_bps))
            .unwrap_or_default(),
    );

    reqwest::get(url)
        .await?
        .json()
        .await
        .map_err(|err| err.into())
}

/// Get swap serialized transactions for a quote
pub async fn swap(quote: Quote, user_public_key: Pubkey) -> Result<Swap> {
    let url = "https://quote-api.jup.ag/v1/swap";

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(non_snake_case)]
    struct SwapRequest {
        quote: Quote,
        wrap_unwrap_SOL: bool,
        fee_account: Option<String>,
        token_ledger: Option<String>,
        user_public_key: String,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct SwapResponse {
        setup_transaction: Option<String>,
        swap_transaction: String,
        cleanup_transaction: Option<String>,
    }

    let request = SwapRequest {
        quote,
        wrap_unwrap_SOL: true,
        fee_account: None,
        token_ledger: None,
        user_public_key: user_public_key.to_string(),
    };

    let response = reqwest::Client::builder()
        .build()?
        .post(url)
        .json(&request)
        .send()
        .await?
        .json::<SwapResponse>()
        .await?;

    fn decode(base64_transaction: String) -> Result<Transaction> {
        bincode::deserialize(&base64::decode(base64_transaction)?).map_err(|err| err.into())
    }

    Ok(Swap {
        setup: response.setup_transaction.map(decode).transpose()?,
        swap: decode(response.swap_transaction)?,
        cleanup: response.cleanup_transaction.map(decode).transpose()?,
    })
}

/// Returns a hash map, input mint as key and an array of valid output mint as values
pub async fn route_map(only_direct_routes: bool) -> Result<RouteMap> {
    let url = format!(
        "https://quote-api.jup.ag/v1/indexed-route-map?onlyDirectRoutes={}",
        only_direct_routes
    );

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct IndexedRouteMap {
        mint_keys: Vec<String>,
        indexed_route_map: HashMap<usize, Vec<usize>>,
    }

    let response = reqwest::get(url).await?.json::<IndexedRouteMap>().await?;

    let mint_keys = response
        .mint_keys
        .into_iter()
        .map(|x| x.parse::<Pubkey>().map_err(|err| err.into()))
        .collect::<Result<Vec<Pubkey>>>()?;

    let mut route_map = HashMap::new();
    for (from_index, to_indices) in response.indexed_route_map {
        route_map.insert(
            mint_keys[from_index],
            to_indices.into_iter().map(|i| mint_keys[i]).collect(),
        );
    }

    Ok(route_map)
}
