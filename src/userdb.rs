use serde::{Deserialize, Serialize};
use serenity::model::prelude::*;
use std::collections::HashMap;
use tracing::{error, info, warn};

pub fn analyze_message(msg: &str) -> SentimentResult {
    thread_local! {
        static ANALYZER: vader_sentiment::SentimentIntensityAnalyzer<'static> =
                         vader_sentiment::SentimentIntensityAnalyzer::new();
    }

    ANALYZER.with(|analyzer| {
        SentimentResult::from_hashmap(analyzer.polarity_scores(msg))
    })
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SentimentResult {
    negative: f64,
    neutral: f64,
    positive: f64,
    compound: f64,
}

impl SentimentResult {
    pub fn from_hashmap(analysis: HashMap<&str, f64>) -> SentimentResult {
        SentimentResult {
            negative: analysis["neg"],
            neutral: analysis["neu"],
            positive: analysis["pos"],
            compound: analysis["compound"],
        }
    }

    pub fn to_formatted_json(&self) -> String {
        format!(
            "{{\n  \
                \"negative\": {},\n  \
                \"neutral\": {},\n  \
                \"positive\": {},\n  \
                \"compound\": {}\n\
                }}",
            self.negative, self.neutral, self.positive, self.compound
        )
    }
}

pub struct ComputedData {
    pub crime_coefficient: f64,
    // TODO: Add Hue
}

impl ComputedData {
    pub fn from_sentiment_values(
        values: &Vec<SentimentResult>,
    ) -> ComputedData {
        let mut total = 0.0;
        for sentiment in values.iter() {
            total += sentiment.compound;
        }
        let average_compound = total / values.len() as f64;

        ComputedData {
            crime_coefficient: ComputedData::convert_compound_to_cc(
                average_compound,
            ),
        }
    }

    pub fn convert_compound_to_cc(compound: f64) -> f64 {
        const CC_NEUTRAL: f64 = 75.0;
        const CC_POLY3: f64 = 1.0;
        const CC_POLY2: f64 = 2.0;
        const CC_POLY1: f64 = 5.0;

        if compound >= 0.0 {
            (1.0 - compound) * CC_NEUTRAL
        } else {
            let base_factor = (1.0 - compound.abs()).recip();
            (base_factor * CC_POLY3).powi(3)
                + (base_factor * CC_POLY2).powi(2) * CC_POLY1
                + CC_NEUTRAL
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserProfileData {
    user_tag: String,
    sentiment_values: Vec<SentimentResult>,
    oldest_index: usize,
}

impl UserProfileData {
    const MAX_USER_HISTORY: usize = 100;

    fn new(tag: &str) -> UserProfileData {
        UserProfileData {
            user_tag: tag.to_string(),
            sentiment_values: Vec::new(),
            oldest_index: 0,
        }
    }

    pub fn get_cymatic_data(&self) -> ComputedData {
        ComputedData::from_sentiment_values(&self.sentiment_values)
    }

    fn add_sentiment_result(&mut self, result: SentimentResult) {
        if self.sentiment_values.len() < UserProfileData::MAX_USER_HISTORY {
            self.sentiment_values.push(result);
        } else {
            // overwrite the oldest value
            self.sentiment_values[self.oldest_index] = result;
            self.oldest_index =
                (self.oldest_index + 1) % UserProfileData::MAX_USER_HISTORY;
        }
    }

    pub fn delete_data(&mut self) {
        self.sentiment_values.clear();
        self.oldest_index = 0;
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserProfilesDatabase {
    db: HashMap<UserId, UserProfileData>,

    #[serde(skip)]
    dirty: bool,
}

const USERDB_FILE: &str = "userccs.db";

impl UserProfilesDatabase {
    pub fn try_create_from_disk() -> UserProfilesDatabase {
        let result = std::fs::read_to_string(USERDB_FILE);

        match result {
            Ok(contents) => {
                let mut db: UserProfilesDatabase =
                    serde_json::from_str(&contents).unwrap();
                info!("Database successfully loaded from '{}'", USERDB_FILE);
                db.dirty = false;
                db
            }
            Err(why) => {
                if why.kind() == std::io::ErrorKind::NotFound {
                    warn!(
                        "Database named '{}' not found, creating new.",
                        USERDB_FILE
                    );
                    UserProfilesDatabase {
                        db: HashMap::new(),
                        dirty: false,
                    }
                } else {
                    panic!("Failed to open file '{}': {}", USERDB_FILE, why);
                }
            }
        }
    }

    pub fn to_disk(&mut self) {
        if !self.dirty {
            return;
        }

        let serialized = serde_json::to_string(&self).unwrap();
        if let Err(why) = std::fs::write(USERDB_FILE, serialized) {
            error!("Failed to save database: {}", why);
        } else {
            info!("Successfully saved database.");
            self.dirty = false;
        }
    }

    pub fn add_sentiment_result_for_user(
        &mut self,
        user: &User,
        result: SentimentResult,
    ) {
        let profile_data = self
            .db
            .entry(user.id)
            .or_insert(UserProfileData::new(&user.tag()));
        profile_data.add_sentiment_result(result);
        self.dirty = true;
    }

    pub fn get_user_profile(&self, id: &UserId) -> Option<&UserProfileData> {
        self.db.get(id)
    }

    pub fn get_user_profile_as_mut(
        &mut self,
        id: &UserId,
    ) -> Option<&mut UserProfileData> {
        self.dirty = true;
        self.db.get_mut(id)
    }
}
