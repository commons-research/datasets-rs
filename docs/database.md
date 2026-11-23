# Database

The datasets are stored in an sqlite file within the repository root named `datasets.sqlite`. 

The table consists of 5 tables:

1. `crawl_run` - Crawl execution history with time stamps.

2. `repository` - The list of all discovered repositories and date of last visit.

3. `artifact` - The files found in the repository that may contain valid datasets.

4. `classification` - List of positive scientific classification evidence per `artifact`.

5. `failure` - Recorded inspection errors and relevant information (e.g. parsing failure). 

