//! Загрузка необязательной внешней таблицы тарифов и точная кредитная арифметика.
//!
//! Тарифы хранятся как миллионные доли кредита за миллион токенов. Умножение такого значения на
//! число токенов даёт пикокредиты: стоимости складываются без погрешности двоичного `float` и
//! округляются только при отображении.

use crate::AbsolutePathBuf;
use serde::Deserialize;
use serde_json::Number;
use serde_json::Value;
use std::collections::BTreeMap;
use std::io;
use std::io::ErrorKind;

const RATE_SCALE: u64 = 1_000_000;
const PICOCREDITS_PER_CREDIT: u128 = 1_000_000_000_000;
const PICOCREDITS_PER_CENT: u128 = PICOCREDITS_PER_CREDIT / 100;

/// Точная неотрицательная кредитная стоимость одного или нескольких вызовов модели.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CreditAmount {
    picocredits: u128,
}

impl CreditAmount {
    /// Складывает две стоимости без переполнения агрегата на повреждённых данных.
    pub fn saturating_add(self, other: Self) -> Self {
        Self {
            picocredits: self.picocredits.saturating_add(other.picocredits),
        }
    }

    /// Возвращает точное внутреннее значение: один кредит равен триллиону пикокредитов.
    pub fn picocredits(self) -> u128 {
        self.picocredits
    }

    /// Отличает положительную стоимость меньше сотой кредита от настоящего нуля до округления.
    pub fn is_positive_below_one_cent(self) -> bool {
        self.picocredits > 0 && self.picocredits < PICOCREDITS_PER_CENT
    }

    /// Округляет до ближайшей сотой кредита после всего необходимого сложения.
    pub fn rounded_hundredths(self) -> u128 {
        self.picocredits.saturating_add(PICOCREDITS_PER_CENT / 2) / PICOCREDITS_PER_CENT
    }
}

/// Эффективное состояние кредитных тарифов в runtime-конфигурации.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum CreditRatesState {
    /// `credit_rates_path` не настроен, поэтому вывод токенов остаётся прежним.
    #[default]
    Disabled,
    /// Настроенный файл не удалось загрузить либо нарушен его верхнеуровневый контракт.
    Unavailable,
    /// Таблица загружена; отсутствующие или отклонённые slug моделей остаются неизвестными.
    Loaded(CreditRates),
}

impl CreditRatesState {
    /// Нужно ли добавлять к модельным строкам кредитную оценку, включая неизвестную `?Ƶ`.
    pub fn is_enabled(&self) -> bool {
        !matches!(self, Self::Disabled)
    }

    /// Вычисляет точную стоимость модели при наличии полной тарифной записи.
    pub fn cost_for_model(
        &self,
        model: &str,
        non_cached_input: u64,
        cached_input: u64,
        output: u64,
    ) -> Option<CreditAmount> {
        let Self::Loaded(rates) = self else {
            return None;
        };
        rates.cost_for_model(model, non_cached_input, cached_input, output)
    }
}

/// Корректные тарифы моделей из одной внешней таблицы.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CreditRates {
    models: BTreeMap<String, ModelCreditRates>,
}

impl CreditRates {
    /// Вычисляет стоимость модели без догадок и сопоставления slug по префиксу.
    pub fn cost_for_model(
        &self,
        model: &str,
        non_cached_input: u64,
        cached_input: u64,
        output: u64,
    ) -> Option<CreditAmount> {
        self.models
            .get(model)
            .map(|rates| rates.cost(non_cached_input, cached_input, output))
    }
}

/// Пригодная таблица и независимо отклонённые при проверке записи моделей.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreditRatesLoad {
    pub rates: CreditRates,
    pub invalid_model_entries: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CreditRate {
    scaled_per_million: u64,
}

impl CreditRate {
    fn cost(self, tokens: u64) -> CreditAmount {
        // Тариф в миллионных долях кредита после деления на миллион токенов оставляет
        // произведение в пикокредитах. Сохранение числителя исключает преждевременное деление.
        CreditAmount {
            picocredits: u128::from(tokens).saturating_mul(u128::from(self.scaled_per_million)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ModelCreditRates {
    input: CreditRate,
    cached_input: CreditRate,
    output: CreditRate,
}

impl ModelCreditRates {
    fn cost(self, non_cached_input: u64, cached_input: u64, output: u64) -> CreditAmount {
        self.input
            .cost(non_cached_input)
            .saturating_add(self.cached_input.cost(cached_input))
            .saturating_add(self.output.cost(output))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCreditRatesFile {
    schema_version: u64,
    unit: String,
    #[serde(rename = "source")]
    _source: Option<String>,
    models: BTreeMap<String, Value>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawModelCreditRates {
    input: Number,
    cached_input: Number,
    output: Number,
}

/// Читает и разбирает один явно настроенный внешний файл кредитных тарифов.
pub fn load_credit_rates(path: &AbsolutePathBuf) -> io::Result<CreditRatesLoad> {
    let contents = std::fs::read_to_string(path).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!(
                "failed to read credit_rates_path `{}`: {err}",
                path.display()
            ),
        )
    })?;
    parse_credit_rates(&contents).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!(
                "failed to parse credit_rates_path `{}`: {err}",
                path.display()
            ),
        )
    })
}

/// Разбирает версионированный документ, отделяя повреждённые записи моделей от корректных.
pub fn parse_credit_rates(contents: &str) -> io::Result<CreditRatesLoad> {
    let raw = serde_json::from_str::<RawCreditRatesFile>(contents).map_err(|err| {
        io::Error::new(
            ErrorKind::InvalidData,
            format!("invalid JSON document: {err}"),
        )
    })?;
    let RawCreditRatesFile {
        schema_version,
        unit,
        _source: _,
        models,
    } = raw;
    if schema_version != 1 {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            format!("unsupported schema_version {schema_version}; expected 1"),
        ));
    }
    if unit != "credits_per_million_tokens" {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            format!("unsupported unit `{unit}`; expected `credits_per_million_tokens`"),
        ));
    }

    let mut valid_models = BTreeMap::new();
    let mut invalid_model_entries = Vec::new();
    for (model, value) in models {
        let parsed = serde_json::from_value::<RawModelCreditRates>(value)
            .map_err(|err| format!("invalid tariff object: {err}"))
            .and_then(ModelCreditRates::try_from);
        match parsed {
            Ok(rates) if !model.is_empty() => {
                valid_models.insert(model, rates);
            }
            Ok(_) => invalid_model_entries.push("empty model slug".to_string()),
            Err(err) => invalid_model_entries.push(format!("`{model}`: {err}")),
        }
    }

    Ok(CreditRatesLoad {
        rates: CreditRates {
            models: valid_models,
        },
        invalid_model_entries,
    })
}

impl TryFrom<RawModelCreditRates> for ModelCreditRates {
    type Error = String;

    fn try_from(raw: RawModelCreditRates) -> Result<Self, Self::Error> {
        Ok(Self {
            input: parse_rate("input", &raw.input)?,
            cached_input: parse_rate("cached_input", &raw.cached_input)?,
            output: parse_rate("output", &raw.output)?,
        })
    }
}

/// Переводит неотрицательное JSON-число максимум с шестью знаками после точки в fixed point.
fn parse_rate(field: &str, number: &Number) -> Result<CreditRate, String> {
    let text = number.to_string();
    if text.starts_with('-') {
        return Err(format!("`{field}` must be non-negative"));
    }
    if text.contains(['e', 'E']) {
        return Err(format!(
            "`{field}` must use ordinary decimal notation with at most six fractional digits"
        ));
    }
    let (whole, fractional_digits) = text.split_once('.').unwrap_or((&text, ""));
    if fractional_digits.len() > 6 {
        return Err(format!("`{field}` has more than six fractional digits"));
    }
    let whole = whole
        .parse::<u64>()
        .map_err(|_| format!("`{field}` is outside the supported range"))?;
    let fractional = if fractional_digits.is_empty() {
        0
    } else {
        fractional_digits
            .parse::<u64>()
            .map_err(|_| format!("`{field}` is not a decimal number"))?
    };
    let fractional_multiplier = 10_u64.pow(6 - fractional_digits.len() as u32);
    let scaled_per_million = whole
        .checked_mul(RATE_SCALE)
        .and_then(|scaled| {
            fractional
                .checked_mul(fractional_multiplier)
                .and_then(|fractional| scaled.checked_add(fractional))
        })
        .ok_or_else(|| format!("`{field}` is outside the supported range"))?;
    Ok(CreditRate { scaled_per_million })
}

#[cfg(test)]
#[path = "credit_rates_tests.rs"]
mod tests;
