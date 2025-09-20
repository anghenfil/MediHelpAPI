use std::sync::Arc;
use chrono::NaiveDate;
use regex::Regex;
use rocket::form::validate::Contains;
use rocket::serde::Serialize;
use crate::TempStorage;
use scraper::*;

#[derive(Debug, Clone, Serialize)]
pub struct Brief{
    pub letter_type: LetterType,
    pub source: LetterSource,
    pub date: NaiveDate,
    pub title: String,
    pub wirkstoffe: Option<Vec<String>>,
    pub link_to_html: String,
    pub link_to_pdf: String,
    pub short_description: Option<String>,
    pub long_description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub enum LetterType{
    RoteHandBrief,
    Informationsbrief
}

#[derive(Debug, Clone, Serialize)]
pub enum LetterSource{
    BfArM,
    PEI
}

pub async fn crawl_bfarm(storage: Arc<TempStorage>) -> Result<(), reqwest::Error> {
    let client = storage.storage.read().await.reqwest_client.clone();

    let mut page = 1;
    let mut briefe : Vec<Brief> = Vec::new();

    loop {
        let mut any_entry_added = false;
        println!("Getting page {}", page);
        let request = client.get(format!("https://www.bfarm.de/DE/Arzneimittel/Pharmakovigilanz/Risikoinformationen/Rote-Hand-Briefe/_node.html?cms_gtp=964792_list%253D{}", page)).build()?;
        let response = client.execute(request).await?;

        let fragment = Html::parse_fragment(&response.text().await?);

        let table_selector = Selector::parse("table").unwrap();
        let table = match fragment.select(&table_selector).next(){
            Some(table) => table,
            None => break,
        };

        let rows_selector = Selector::parse("tr").unwrap();
        let rows : Vec<ElementRef>= table.select(&rows_selector).collect();

        if rows.is_empty() { break; }

        let td_selector = Selector::parse("td").unwrap();
        let a_selector = Selector::parse("a").unwrap();
        let teasertext_selector = Selector::parse("p.teasertext-wrapper").unwrap();

        for row in &rows {
            let tds : Vec<ElementRef>= row.select(&td_selector).collect();

            if tds.len() != 2{
                eprintln!("Expected tr to have two columns. Skipping.");
                continue;
            }

            let mut iter = tds.iter();

            let date = iter.next().unwrap().text().collect::<String >().trim().to_string();
            let date = match NaiveDate::parse_from_str(date.as_str(), "%d.%m.%Y"){
                Ok(date) => date,
                Err(e) => {
                    eprintln!("Couldn't parse letter date: {}", e);
                    continue;
                }
            };

            let datacol = iter.next().unwrap();

            let link = match datacol.select(&a_selector).next(){
                Some(link) => link,
                None => {
                    eprintln!("Expected a link in the data row :( Skipping.");
                    continue;
                },
            };

            let link_to_letter = link.value().attr("href").unwrap();
            let link_to_letter = format!("https://www.bfarm.de/{}", link_to_letter.split(".html").next().unwrap());

            let link_to_pdf = format!("{}?__blob=publicationFile", link_to_letter);
            let title = link.inner_html();

            let p_tag = match datacol.select(&teasertext_selector).next(){
                Some(p_tag) => p_tag,
                None => {
                    eprintln!("Expected a p tag in the data row :( Skipping.");
                    continue;
                }
            };

            let mut short_description = String::new();
            let mut wirkstoffe : Vec<String> = Vec::new();

            for child in p_tag.children(){
                match child.value(){
                    Node::Text(txt) => {
                        short_description += txt;
                    }
                    Node::Element(element) => {
                        if element.name.local.as_ref() == "span" {
                            if element.classes().find(|ele|ele.eq(&"wirkstoff-wrapper")).is_some(){ // Found wirkstoff wrapper
                                let el_ref = ElementRef::wrap(child).unwrap();
                                let mut span_text = el_ref.text().collect::<String>();
                                span_text = match span_text.split("Wirkstoff:").last(){
                                    None => {
                                        eprintln!("No Wirkstoff found in span.");
                                        continue;
                                    }
                                    Some(wirkstoff) => wirkstoff.to_string(),
                                };
                                wirkstoffe = span_text.split(|c| c == ',' || c == '/').map(|ele|ele.trim().to_string()).collect();
                            }
                        }else {
                            let el_ref = ElementRef::wrap(child).unwrap();
                            short_description += &el_ref.text().collect::<String>();
                        }
                    },
                    _ => {

                    }
                }
            }

            let short_description = short_description.trim().to_string();

            let temp = title.to_lowercase();
            let letter_type = if temp.contains("rote-hand-brief") || temp.contains("rote hand brief")|| temp.contains("rote-hand brief"){
                LetterType::RoteHandBrief
            }else{
                LetterType::Informationsbrief
            };
            
            let brief = Brief{
                letter_type,
                source: LetterSource::BfArM,
                date,
                title,
                wirkstoffe: Some(wirkstoffe),
                link_to_html: link_to_letter,
                link_to_pdf,
                short_description: Some(short_description),
                long_description: None,
            };
            briefe.push(brief);
            any_entry_added = true;
        }

        if !any_entry_added {
            break;
        }
        
        page = page+1;
    }

    for mut brief in briefe.into_iter() {
        if storage.storage.read().await.briefe.contains_key(&brief.link_to_html){
            println!("Skipping letter, already in database.");
            continue;
        }

        brief.long_description = bfarm_crawl_detailed_entry(client.clone(), &brief.link_to_html).await?;
        println!("Added new letter from BfArM ({})", &brief.title);
        storage.storage.write().await.briefe.insert(brief.link_to_html.clone(), brief);
    }

    println!("Finished crawl!");

    Ok(())
}

async fn bfarm_crawl_detailed_entry(client: reqwest::Client, url: &str) -> Result<Option<String>, reqwest::Error> {
    let request = client.get(url).build()?;
    let response = client.execute(request).await?;

    let fragment = Html::parse_fragment(&response.text().await?);

    let description_p_tag_selector = Selector::parse(".content > p").unwrap();
    match fragment.select(&description_p_tag_selector).next(){
        None => Ok(None),
        Some(p) => Ok(Some(p.text().collect::<String>().trim().to_string())),
    }
}

pub async fn crawl_pei(storage: Arc<TempStorage>) -> Result<(), reqwest::Error>{
    let client = storage.storage.read().await.reqwest_client.clone();

    let mut page = 1;
    let mut brief_links: Vec<String> = Vec::new();

    loop{
        let request = client.get(format!("https://www.pei.de/SiteGlobals/Forms/Suche/Sicherheitsinformationsuche_Formular.html?input_=170452&gtp=213258_list%253D{}&resourceId=211336&submit.x=22&submit.y=14&templateQueryString=&sortOrder=score+desc&pageLocale=de", page)).build()?;
        let response = client.execute(request).await?;

        let fragment = Html::parse_fragment(&response.text().await?);
        let selector = Selector::parse(".searchresult > .teaser a").unwrap();

        let searchresults = fragment.select(&selector).collect::<Vec<ElementRef>>();
        if searchresults.len() == 0 {
            break;
        }
        
        for link in searchresults {
            let title = link.text().collect::<String>().trim().to_lowercase();
            if let Some(link_href) = link.attr("href"){
                if title.contains("rote-hand-brief") || title.contains("rote-hand brief") || title.contains("rote hand brief") || title.contains("informationsbrief"){
                    brief_links.push(format!("https://www.pei.de/{}", link_href));
                }
            }
        }

        page = page+1;
    }

    println!("Found {} PEI letters. Crawling details...", brief_links.len());

    let mut letters = Vec::new();
    for brief_link in brief_links {
        if !storage.storage.read().await.briefe.contains_key(&brief_link){
            if let Some(letter) = pei_crawl_detailed_entry(client.clone(), &brief_link).await? {
                letters.push(letter);
            }else{
                println!("Failed to crawl pei letter: {}", brief_link);
            }
        }
    }
    for letter in letters {
        println!("Added letter: {:?}", letter);
        storage.storage.write().await.briefe.insert(letter.link_to_html.clone(), letter);
    }

    println!("Finished crawl for PEI");
    Ok(())
}

async fn pei_crawl_detailed_entry(client: reqwest::Client, url: &str) -> Result<Option<Brief>, reqwest::Error> {
    let request = client.get(url).build()?;
    let response = client.execute(request).await?;

    let fragment = Html::parse_fragment(&response.text().await?);

    let title_selector = Selector::parse(".content > h1").unwrap();
    let title = match fragment.select(&title_selector).next(){
        None => return Ok(None),
        Some(title) => {
            title.text().collect::<String>().trim().to_string()
        }
    };
    let temp = title.to_lowercase();
    let letter_type = if temp.contains("rote-hand-brief") || temp.contains("rote-hand brief") || temp.contains("rote hand brief"){
        LetterType::RoteHandBrief
    }else{
        LetterType::Informationsbrief
    };
    let description_short = match fragment.select(&Selector::parse(".content > .abstract > p").unwrap()).next(){
        None => None,
        Some(p_tag) => {
            let mut description = String::new();
            for child in p_tag.children(){
                match child.value(){
                    Node::Text(txt) => {
                        description += txt;
                    }
                    Node::Element(_) => {
                        let el_ref = ElementRef::wrap(child).unwrap();
                        description += &el_ref.text().collect::<String>();
                    },
                    _ => {}
                }
            }
            Some(description)
        }
    };
    let download_a_tag = match fragment.select(&Selector::parse(".content a").unwrap()).next(){
        None => return Ok(None),
        Some(a_tag) => {
            a_tag
        }
    };

    let pdf_link = match download_a_tag.value().attr("href"){
        None => return Ok(None),
        Some(href) => format!("https://www.pei.de{}", href),
    };

    let download_link_text = download_a_tag.text().collect::<String>();
    let regex = Regex::new(r"\((\d{2}\.\d{2}\.\d{4})\)").unwrap();

    let date: NaiveDate = if let Some(caps) = regex.captures(download_link_text.as_str()) {
        match NaiveDate::parse_from_str(&caps[1], "%d.%m.%Y"){
            Ok(date) => date,
            Err(e) => {
                eprintln!("Couldn't parse letter date: {}", e);
                return Ok(None);
            }
        }
    }else{
        // Second try to get date via updating date
        match fragment.select(&Selector::parse(".c-date__created > p").unwrap()).next(){
            None => return Ok(None),
            Some(date_str) => {
                let date_str = date_str.text().collect::<String>().trim().to_string();
                match date_str.split("Aktualisiert:").last(){
                    None => return Ok(None),
                    Some(date_str) => {
                        println!("Warning: using create date since no publishing date was found.");
                        match NaiveDate::parse_from_str(date_str, "%d.%m.%Y"){
                            Ok(date) => date,
                            Err(e) => {
                                eprintln!("Couldn't parse letter date: {}", e);
                                return Ok(None);
                            }
                        }
                    }
                }
            }
        }
    };

    Ok(Some(Brief{
        letter_type,
        source: LetterSource::PEI,
        date,
        title,
        wirkstoffe: None,
        link_to_html: url.to_string(),
        link_to_pdf: pdf_link,
        short_description: description_short,
        long_description: None,
    }))
}