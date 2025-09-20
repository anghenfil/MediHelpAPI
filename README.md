Scraper for Rote-Hand-Briefe & drug supply shortages in Germany.

Currently, there are only 2 simple API endpoints which return all data (/api/lieferengpaesse and /api/briefe), filtering and sorting options will be added soon™

The scraper is written in rust and will scrape the websites of the Paul-Ehrlich-Institut (PEI) and Bundesinstitut für Arzneimittel und Medizinprodukte (BfArM) once and re visit the websites every few minutes to fetch updates. All data is stored in memory only.

A public instance is available at https://api.medihelp.app (-> https://api.medihelp.app/api/lieferengpaesse and https://api.medihelp.app/api/briefe).
