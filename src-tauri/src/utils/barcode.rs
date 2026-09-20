use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ParsedBarcode {
    pub raw_barcode: String,
    pub is_weight_embedded: bool,
    pub item_code: String,
    pub weight: Option<Decimal>,
    pub embedded_amount: Option<Decimal>,
}

pub fn parse_barcode(barcode: &str) -> ParsedBarcode {
    let trimmed = barcode.trim();

    // Variable weight barcode formats:
    // Format A (Prefix 1 digit '2'): e.g. 2 + 5 digits item (00001) + 5 digits weight in grams (01250 = 1.250kg) + 1 check digit
    // Format B (Prefix 2 digits '20' or '02'): e.g. 20 + 5 digits item + 5 digits weight + 1 check digit
    if (trimmed.len() == 12 || trimmed.len() == 13) && trimmed.starts_with('2') {
        // Try Format A (Prefix 2 + 5 digits SKU + 5 digits weight)
        if trimmed.len() >= 12 {
            let code_part = &trimmed[1..6];
            let value_part = &trimmed[6..11];
            if let Ok(int_val) = value_part.parse::<i64>() {
                if int_val > 0 {
                    let weight_kg = Decimal::new(int_val, 3);
                    return ParsedBarcode {
                        raw_barcode: trimmed.to_string(),
                        is_weight_embedded: true,
                        item_code: code_part.to_string(),
                        weight: Some(weight_kg),
                        embedded_amount: None,
                    };
                }
            }
        }

        // Try Format B (Prefix 20/02 + 5 digits SKU + 5 digits weight)
        if trimmed.len() == 13 && (trimmed.starts_with("20") || trimmed.starts_with("02")) {
            let code_part = &trimmed[2..7];
            let value_part = &trimmed[7..12];
            if let Ok(int_val) = value_part.parse::<i64>() {
                if int_val > 0 {
                    let weight_kg = Decimal::new(int_val, 3);
                    return ParsedBarcode {
                        raw_barcode: trimmed.to_string(),
                        is_weight_embedded: true,
                        item_code: code_part.to_string(),
                        weight: Some(weight_kg),
                        embedded_amount: None,
                    };
                }
            }
        }
    }

    ParsedBarcode {
        raw_barcode: trimmed.to_string(),
        is_weight_embedded: false,
        item_code: trimmed.to_string(),
        weight: None,
        embedded_amount: None,
    }
}
