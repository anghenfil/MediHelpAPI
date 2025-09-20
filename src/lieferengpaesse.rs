use std::num::ParseIntError;
use std::sync::Arc;
use chrono::NaiveDate;
use rocket::serde::Deserialize;
use serde::Serialize;
use crate::TempStorage;

pub async fn refresh_lieferengpaesse(storage: Arc<TempStorage>) -> Result<(), reqwest::Error>{
    let client = storage.storage.read().await.reqwest_client.clone();
    // Get csv
    let request = client.get("https://anwendungen.pharmnet-bund.de/lieferengpassmeldungen/public/csv").build()?;
    let response = client.execute(request).await?.text_with_charset("WINDOWS-1252").await?;


    // Parse csv
    let mut rdr = csv::ReaderBuilder::new().flexible(false).delimiter(b';').from_reader(response.as_bytes());

    let mut results = Vec::new();
    for result in rdr.deserialize() {
        // We must tell Serde what type we want to deserialize into.
        let record: Lieferengpass = match result{
            Ok(record) => record,
            Err(error) => {
                eprintln!("Cant parse record: ");
                eprintln!("{}", error);
                continue;
            }
        };
        results.push(record);
    }
    storage.storage.write().await.lieferengpaesse = results;
    println!("Refreshed Lieferengpässe.");
    Ok(())
}

pub fn deserialize_na_option<'de, D>(deserializer: D) -> Result<Option<String>, D::Error> where D: serde::Deserializer<'de>{
    let mut raw = String::deserialize(deserializer)?;
    raw = raw.trim().to_string();

    if raw.to_lowercase() == "n/a" || raw.is_empty(){
        Ok(None)
    }else{
        Ok(Some(raw.to_string()))
    }
}

fn de_date<'de, D>(d: D) -> Result<NaiveDate, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(d)?;
    NaiveDate::parse_from_str(s.trim(), "%d.%m.%Y")
        .map_err(serde::de::Error::custom)
}

fn bool_ja_nein<'de, D>(d: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(d)?;
    match s.trim().to_ascii_lowercase().as_str() {
        "ja" | "true" | "1" => Ok(true),
        "nein" | "false" | "0" => Ok(false),
        other => Err(serde::de::Error::custom(format!("Bool erwartet Ja/Nein, bekam: {other}"))),
    }
}

fn de_enrs<'de, D>(d: D) -> Result<Vec<usize>, D::Error> where D: serde::Deserializer<'de>{
    let s = String::deserialize(d)?;

    let parts = s.split(",");
    let enrs: Result<Vec<usize>, ParseIntError> = parts.map(|enr|enr.trim().parse::<usize>()).collect();
    match enrs {
        Ok(enrs) => Ok(enrs),
        Err(e) => Err(serde::de::Error::custom(format!("{}", e))),
    }
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct Lieferengpass{
    #[serde(rename(deserialize  = "PZN"))]
    pub pzn: usize,
    #[serde(rename(deserialize = "ENR"), deserialize_with = "de_enrs")]
    pub enr: Vec<usize>,
    #[serde(rename(deserialize = "Bearbeitungsnummer"))]
    pub bearbeitungsnummer: String,
    #[serde(rename(deserialize = "Referenzierte Erstmeldung"), deserialize_with = "deserialize_na_option")]
    pub erstmeldung: Option<String>,
    #[serde(rename(deserialize = "Datum der Erstmeldung"), deserialize_with = "de_date")]
    pub erstmeldung_datum: NaiveDate,
    #[serde(rename(deserialize = "Meldungsart"))]
    pub meldungsart: Meldungsart,
    #[serde(rename(deserialize = "Beginn"), deserialize_with = "de_date")]
    pub beginn: NaiveDate,
    #[serde(rename(deserialize = "Ende"), deserialize_with = "de_date")]
    pub ende: NaiveDate,
    #[serde(rename(deserialize = "Datum der letzten Meldung"), deserialize_with = "de_date")]
    pub letzte_meldung: NaiveDate,
    #[serde(rename(deserialize = "Art des Grundes"))]
    pub art_des_grundes: ArtDesGrundes,
    #[serde(rename(deserialize = "Arzneimittlbezeichnung"))]
    pub arzneimittelbezeichnung: String,
    #[serde(rename(deserialize = "Atc Code"))]
    pub atc: String,
    #[serde(rename(deserialize = "Wirkstoffe"))]
    pub wirkstoffe: String,
    #[serde(rename(deserialize = "Krankenhausrelevant"), deserialize_with = "bool_ja_nein")]
    pub kkh_relevant: bool,
    #[serde(rename(deserialize = "Zulassungsinhaber"))]
    pub zulassungsinhaber: String,
    #[serde(rename(deserialize = "Grund"))]
    pub grund: String,
    #[serde(rename(deserialize = "Anm. zum Grund"), deserialize_with = "deserialize_na_option")]
    pub anmerkung_zum_grund: Option<String>,
    #[serde(rename(deserialize = "Alternativpräparat"), deserialize_with = "deserialize_na_option")]
    pub alternativpraeparat: Option<String>,
    #[serde(rename(deserialize = "Info an Fachkreise"))]
    pub info_an_fachkreise: InfoAnFachkreise,
    #[serde(rename(deserialize = "Darreichungsform"))]
    pub darreichungsform: String,
    #[serde(rename(deserialize = "klassifikation"))]
    pub klassifikation: Klassifikation,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum Klassifikation{
    #[serde(rename = "weder versrel noch verskri")]
    WederVersorgungsrelevantNochVersorgungskritisch,
    #[serde(rename = "versrel")]
    Versorgungsrelevant,
    #[serde(rename = "verskri (auch versrel)")]
    VersorgungsrelevantAuchVersorgungskritisch
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum InfoAnFachkreise{
    Nein,
    Ja,
    Vorgesehen,
    #[serde(rename = "N/A")]
    Unbekannt
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum ArtDesGrundes{
    Produktionsproblem,
    Sonstige
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Meldungsart{
    Erstmeldung,
    #[serde(rename = "Änderungsmeldung")]
    Aenderungsmeldung,
    #[serde(rename = "Löschmeldung")]
    Loeschmeldung
}