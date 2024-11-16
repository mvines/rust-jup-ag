// Deserialize Instruction with a custom function
pub mod instruction {
    use base64::prelude::{Engine as _, BASE64_STANDARD};
    use serde::{Deserialize, Deserializer};
    use solana_sdk::{instruction::AccountMeta, instruction::Instruction, pubkey::Pubkey};
    use std::str::FromStr;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Instruction, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct InstructionFields {
            accounts: Vec<AccountMetaFields>,
            data: String,
            program_id: String,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct AccountMetaFields {
            pubkey: String,
            is_signer: bool,
            is_writable: bool,
        }

        let fields = InstructionFields::deserialize(deserializer)?;
        let program_id = Pubkey::from_str(&fields.program_id)
            .map_err(|e| serde::de::Error::custom(format!("Error parsing programId: {}", e)))?;

        let accounts = fields
            .accounts
            .into_iter()
            .map(|acc| {
                let pubkey = Pubkey::from_str(&acc.pubkey).map_err(|e| {
                    serde::de::Error::custom(format!("Error parsing pubkey: {}", e))
                })?;
                Ok(AccountMeta {
                    pubkey,
                    is_signer: acc.is_signer,
                    is_writable: acc.is_writable,
                })
            })
            .collect::<Result<Vec<AccountMeta>, _>>()?;

        let instruction = Instruction {
            program_id,
            accounts,
            data: BASE64_STANDARD
                .decode(&fields.data)
                .map_err(|e| serde::de::Error::custom(format!("Error decoding data: {}", e)))?,
        };

        Ok(instruction)
    }
}

// Deserialize Option<Instruction> with a custom function
pub mod option_instruction {
    use serde::{Deserialize, Deserializer};
    use solana_sdk::instruction::Instruction;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Instruction>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: serde_json::Value = Deserialize::deserialize(deserializer)?;

        match value {
            serde_json::Value::Null => Ok(None),
            _ => crate::field_instruction::instruction::deserialize(value)
                .map_err(|e| {
                    serde::de::Error::custom(format!(
                        "Error deserialize optional instruction: {}",
                        e
                    ))
                })
                .map(Some),
        }
    }
}

// Deserialize Vec<Instruction> with a custom function
pub mod vec_instruction {
    use serde::{Deserialize, Deserializer};
    use solana_sdk::instruction::Instruction;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<Instruction>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let values: Vec<serde_json::Value> = Deserialize::deserialize(deserializer)?;
        let mut instructions = Vec::new();

        for value in values {
            let instruction: Instruction =
                crate::field_instruction::instruction::deserialize(value).map_err(|e| {
                    serde::de::Error::custom(format!("Error deserialize vec instruction: {}", e))
                })?;
            instructions.push(instruction);
        }

        Ok(instructions)
    }
}
