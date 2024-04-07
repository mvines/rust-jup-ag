pub mod vec {
    use {
        serde::{de, Deserializer, Serializer},
        serde::{Deserialize, Serialize},
        solana_sdk::pubkey::Pubkey,
        std::str::FromStr,
    };

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<Pubkey>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec_str: Vec<String> = Vec::deserialize(deserializer)?;
        let mut vec_pubkey = Vec::new();
        for s in vec_str {
            let pubkey = Pubkey::from_str(&s).map_err(de::Error::custom)?;
            vec_pubkey.push(pubkey);
        }
        Ok(vec_pubkey)
    }

    #[allow(dead_code)]
    pub fn serialize<S>(vec_pubkey: &[Pubkey], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let vec_str: Vec<String> = vec_pubkey.iter().map(|pubkey| pubkey.to_string()).collect();
        vec_str.serialize(serializer)
    }
}

pub mod option {
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
}
