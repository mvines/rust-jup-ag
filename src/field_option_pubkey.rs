use {
    serde::{Serialize, Serializer},
    solana_sdk::pubkey::Pubkey,
};

pub fn serialize<S>(t: &Option<Pubkey>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match t {
        Some(pubkey) => pubkey.to_string().serialize(serializer),
        None => serializer.serialize_none(),
    }
}