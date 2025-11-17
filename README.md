# What is this

This is Project Themis, a suite of tools which powers [Brier.fyi](https://brier.fyi/) and previously [Calibration City](https://calibration.city/). The purpose of this project is to perform useful analysis of prediction market calibration and accuracy with data from each platform's public API.

# How to run this yourself

## Step 0. Install dependencies

Clone this repository and enter it:

```bash
git clone git@github.com:wasabipesto/themis.git
cd themis
```

Install any other dependencies:

- The downloader and extractor are written in rust. To install the rust toolchain, follow the instructions [here](https://www.rust-lang.org/tools/install). You could run these utilities in Docker but that is not officially supported.
- The website is written with Astro, which uses `node` and `npm`. You can find the official node/npm installation instructions [here](https://docs.npmjs.com/downloading-and-installing-node-js-and-npm), run everything in Docker, or use whatever version Debian stable is shipping.
- [Docker](https://docs.docker.com/engine/install/) and the [docker compose](https://docs.docker.com/compose/install/linux) plugin are used to run the database and its connectors. It's possible to run these without docker by installing [Postgres](https://www.postgresql.org/download/) and [PostgREST](https://docs.postgrest.org/en/stable/tutorials/tut0.html) manually.
- For running tasks I have provided a `justfile`, which requires `just` to run. You can install that by following the instructions [here](https://just.systems/man/en/packages.html). The `justfile` is very simple, and you can just run the commands by hand if you don't want to install it.
- The script for site deployment uses `rclone` and thus can be deployed to any target supported by that utility. You can install rclone by following the instructions [here](https://rclone.org/install/), or deploy the site some other way.
- Some other optional utilities:
  - There are a few Python scripts I use for development in the `scripts` folder. If you want to use these, ensure you have `python` and `uv` [installed](https://docs.astral.sh/uv/getting-started/installation/).
  - When testing API responses I use `jq` for filtering and general formatting. You can get that [here](https://jqlang.org/download/).
  - A couple scripts for debugging are written with `rust-script`. Installation instructions are [here](https://rust-script.org/#installation).
  - Some admin tools lean on an `ollama` API endpoint for extracting keywords, generating slugs, and more. You can find installation instructions [here](https://ollama.com/download). By default it expects that the service will be started and available on localhost.

## Step 1. Downloading API data to disk

In previous versions of this program, we deserialized all API responses immediately upon receiving them in order to work in a type-safe rust environment. This works great if APIs never change. Since external APIs can change unexpectedly, we have broken the download flow into two programs: a downloader and an extractor. The downloader will grab all relevant data from the platform APIs, then the extractor will deserialize that data into something we can use.

Before downloading, make sure you have enough disk space, memory, and time:

- By default the download program will download from all platforms in parallel to avoid getting bottle-necked by any one platform's API rate limit. In order to do this we first download the platform's bulk list as an index and load it into memory. If you are running in the default mode, expect to use around 6 GB of memory. If you run out of memory, you can run the platforms one at a time with the `--platform` option.
- This program will download all relevant data from each platform's API to disk. We try to avoid reading or writing any more than necessary by buffering writes and appending data where possible. Still, a large amount of disk space will be required for this data. As of February 2025 it uses around 20 GB, but this will increase over time.
- When run the first time, this utility takes a day or so to complete. It will first download each platform's index and make a download plan. Then it will queue up batches of downloads that run asynchronously. If you interrupt the program or it runs into an error, simply restart it. It will look for an existing index file and attempt to resume the downloads automatically.

To run the downloader:

```bash
just download --help # for options
just download # run with default settings
```

The download utility is designed to be robust so you can set it and forget it. Errors are much more likely in later steps. If the downloader crashes and resuming a few minutes later does not solve the problem, please [submit an issue](https://github.com/wasabipesto/themis/issues/new). This could be caused by a major shift in a platform's API structure or rate limits.

Note: Do not run multiple instances of the download program to try and make it go faster! Site-specific rate limits are baked in to stay under the rate limits and prevent overloading the servers. The data downloader queues items sequentially, so you will end up with duplicate markets while also getting yourself IP-banned.

## Step 2. Setting up the database

While the downloader is running, set up the database.

First, we'll create our environment file and update the connection passwords.

```bash
cp template.env .env
sed -i "s/^POSTGRES_PASSWORD=.*/POSTGRES_PASSWORD=$(openssl rand -base64 32 | tr -d '/+=' | cut -c1-32)/" .env
sed -i "s/^PGRST_JWT_SECRET=.*/PGRST_JWT_SECRET=$(openssl rand -base64 32 | tr -d '/+=' | cut -c1-32)/" .env
```

Once the `.env` file has been created, you can go in and edit any settings you'd like.

Next, we'll generate our JWT key to authenticate to PostgREST. You can do this with many services, but we'll generate it with this script:

```bash
sed -i "s/^PGRST_APIKEY=.*/PGRST_APIKEY=$(python3 scripts/generate-db-jwt.py)/" .env
```

That key will be valid for 30 days, to refresh it just run that line again.

To actually start our database and associated services:

```bash
just db-up
```

This command will start the database, the REST adapter, and the backup utility. These services need to be running during the `extract` process, group building process, and site building process. When the site is deployed it reaches out to the database for a few non-critical functions.

The database will run in Postgres, which will persist data in the `postgres_data` folder. You should never need to access or edit the contents of this folder. Another container handles backups, which will be placed in the `postgres_backups` folder daily.

If you ever change a setting in the `.env` file, you can re-run `just db-up` to reload the configuration and restart containers if necessary.

To manually run a backup or get the database schema:

```bash
just db-backup # run a backup and save in the postgres_backups folder
just db-get-schema # extract the current schema and output to stdout
```

Import the schema, roles, and some basic data with the `db-load-schema` task:

```bash
just db-load-schema # run all provided SQL files
```

Reload PostgREST for it to see the new schema:

```bash
docker kill -s SIGUSR1 postgrest # trigger a reload
docker restart postgrest # or restart the whole container
```

Then you can test that everything is working with curl:

```bash
just db-curl platforms
```

You should see data for each platform, formatted for readability with `jq`.

We don't need to do this yet, but to completely stop the database and services:

```bash
just db-down
```

## Step 3. Importing data from the cache into the database

Once everything has been downloaded from the platform APIs, we can extract and import that data into the database.

This utility will read the data files you just downloaded and make sure every item matches our known API schemas. If anything changes on the API end, this is where you will see the errors. Please [submit an issue](https://github.com/wasabipesto/themis/issues/new) if you encounter any fatal errors in this step.

Running this program in full on default settings will take about 5 minutes and probably produce a few dozen non-fatal errors. Every platform has a couple items that are "invalid" in some way, and we've taken those into account when setting up our error handling.

After a few thousand items are ready to upload, the program will send them to the database through the PostgREST endpoints. It should fail quickly if it's unable to connect to the database.

Ensure the database services are running and then run:

```bash
just extract --help # for options
just extract --schema-only # check that schemas pass
just extract # run with default settings
```

If you get an error like `Connection refused (os error 111)`, make sure you imported all schemas and reloaded the PostGREST configuration from the previous section.

Then you can test that everything is working with curl:

```bash
just db-curl "markets?limit=10"
just db-curl "markets?select=count"
just db-curl "daily_probabilities?limit=10"
just db-curl "daily_probabilities?select=count"
```

You should see a few sample markets and data points, with total counts for each.

The extract tool is designed to be safe to run multiple times. It will only overwrite items in the market table, and it will update items if they already exist. You can even run it while the download is in-progress to extract what's available.

## Step 4. Creating and processing question groups

The heart of the site are "questions", which are small groups assembled from markets that are equivalent.

Ideally every platform would predict every important event with the same method and resolve with the same criteria, but they don't. Some platforms are legally unable, some have differing market mechanisms, and some just don't like predicting certain things. Our goal is to find a few markets from different platforms that are similar enough to be comparable and link them together under a "question" and do this as many times as possible.

Right now this is done manually in order to ensure that linked markets are actually similar enough to be comparable. For my purposes, two markets are similar enough if the differences in their resolution criteria would affect their probabilities by 1% or less. For instance, two markets with a duration over 6 months with close dates differing by 1 day are usually still similar enough to compare equitably. This requires a fair amount of human judgment, though I am experimenting with ways to automate it.

In the meantime, I have made a secondary Astro site with the tools you will need to search and view markets, link markets into groups, edit the question groups, and edit most other items in the database. To run it in the basic mode, run:

```bash
just group
```

This will launch the site in Astro's dev mode, which will be enough for anything you need to do. The site can also be compiled statically and served in the same way as the main site, but I recommend against doing this since it will have your database admin credentials baked in.

If you want to use embeddings to find similar markets, you can generate them with the following script:

```bash
uv run scripts/update-embeddings.py
```

For now I am intentionally not documenting specific features of the admin tools since they are not user-facing and I am constantly changing them to suit my needs better. The method I have found that works best for me is:

- Sort all markets by volume, number of traders, or duration. Find one that seems interesting.
- Find markets from other platforms that have equivalent or nearly-equivalent resolution criteria.
- Sort those markets by volume, number of traders, or duration to find the one "authoritative" market per platform.
- Create a question group with a representative title and slug consistent with your conventions.
- Add all selected markets to the question by copying in their IDs.
- Check that the probabilities overlap and set start/end date overrides if necessary.
- Check that the resolutions match and invert questions if necessary.
- While you have those searches open, look for other possible question groups in the same topic.
- Once you have exhausted the markets in that topic, return to the top-level search and find another topic.

## Step 5. Site preprocessing

When you have finished grouping markets, you can calculate all market scores by running the grader tool:

```bash
# optional, fix criterion probabilities to be more intuitive for linked questions
uv run scripts/fix-criterion-probs.py

# caluclate all scores and grades
just grade
```

This tool will run through basically everything in the database and calculate some scores that are a little to compute-intensive to do at build time and refresh all the database views. This tool is non-destructive just like the others, you can run it over and over again and lose nothing but your time. Just make sure you re-run it every time you finish grouping markets before generating the site.

You will also need to generate embeddings for related questions. You can generate those with the following script:

```bash
# just generate embeddings for questions
uv run scripts/update-embeddings.py --questions-only

# regenerate embeddings for all items
uv run scripts/update-embeddings.py --all
```


## Step 6. Generating site

The site is static and designed to be deployed behind any standard web server such as `nginx`. It could also be deployed to GitHub Pages, an AWS S3 bucket, or any other static site host.

You can view a preview of the site or build it like so:

```bash
just site-dev # live preview the site in a browser
just site-build # build the site to the site/dist directory
```

The first site page load (in preview mode) or build will take a while as items are downloaded from the server. Subsequent loads/builds will be much faster but will not reflect the database's current state. In order to clear the cache, run the task:

```bash
just site-cache-reset # invalidate site data cache
```

We use `rclone` to deploy the site to your provider of choice. First, configure your `rclone` target and add the details to the `.env` file:

```bash
rclone config # set up a new target
nano .env # add your rclone target path
```

Then, you can deploy the site at any time with this command:

```bash
just deploy # build and deploy site to rclone target
```

### I just want to develop on the site

If you're just developing on the site you don't actually need to use the download and extract tools!

You can build the site against my public database that the main site builds from by doing either of these:

- Change the `PGRST_URL` variable in the `.env` environment file to `https://data.brier.fyi`.
- Run the site development server with `PGRST_URL="https://data.brier.fyi" just site-dev`.

First load of the dev site will be slow while it caches some of the Big Data™. Other than that the Astro project should be pretty straightforward.

## Step 7. Downloading new markets

Over time, new markets will be added and other markets will be updated. In order to update the database with the freshest data, you can re-run the download and extract programs to load the new data.

The download program has two different arguments for resetting:

- `--reset-index` will re-download the platform index and then follow any rules set for what to download. This is good for catching markets that have been added since the last download but will not refresh markets that already existed but were resolved since the last download. This is usually not what you want for updating a database.
- `--reset-cache` used by itself will re-download _everything,_ updating the database with 100% fresh data. Unfortunately this will take several days unless used with one of the filters below.
- `--resolved-since` will filter the market download queue to just those resolved since the given date. Must be in the form of an ISO 8601 string.
- `--resolved-since-days-ago` will do the exact same as the previous option, but with a duration supplied instead of a date. This is usually the best option for a scripted refresh.

All `reset` options make a backup of the previous data files in case you want to look at past data.

```bash
# run a full refresh and add to the database
just download --reset-cache && just extract

# only download markets resolved recently and add to the database
just download --reset-cache --resolved-since-days-ago 10 && just extract
```

After the data is downloaded, you can add groups and edit data in the database as before. Then, build the site again and see the results.

### Wiping the Markets Table

Eventually you may want to wipe the markets table in the database, either because you are changing the database schema or because you want to start fresh. In order to do this without losing data you will need to first export your questions and market-question links. I've provided a script to do this:

```bash
# back up your database, just in case
just db-backup

# export the questions and market links
uv run scripts/migrate.py --mode export

# either drop all tables
just db-run-sql schema/00-drop-all.sql
# or wipe the data folder
just db-down
sudo rm -r postgres_data
just db-up

# load the schema
just db-load-schema

# reload the schema cache
docker kill -s SIGUSR1 postgrest

# import the questions and market links
uv run scripts/migrate.py --mode import

# calculate stats and refresh everything else
just grade

# check and build the site
just site-dev
just site-build

# check everything is in place
just db-curl "market_details?limit=10&question_slug=not.is.null"
```

Note that this is not necessary if you want to edit table views. To reload the database view schema, just run:

```bash
just db-run-sql schema/03-views.sql
```

# I just want the data

The production database is publicly readable via PostgREST at [https://data.brier.fyi](https://data.brier.fyi/). This will lead you to a full OpenAPI spec, which you could plug in to Swagger or your client generator of choice.

For example, to get items from various tables:

```bash
curl -sf https://data.brier.fyi/question_details?limit=100
curl -sf https://data.brier.fyi/market_details?limit=100
curl -sf https://data.brier.fyi/daily_probability_details?limit=100
```

You can find PostgREST documentation here:

- https://docs.postgrest.org/en/stable/references/api/tables_views.html
- https://docs.postgrest.org/en/stable/references/api/pagination_count.html

# Notes, news, and disclaimers

This project has been awarded the following grants:

- $3,500 as part of the [Manifold Community Fund](https://manifund.org/projects/wasabipestos-umbrella-project), an impact certificate-based funding round.
- $8,864 as part of the [EA Community Choice](https://manifund.org/projects/calibration-city), a donation matching pool.

These grants have been used for furthering development but have not influenced the contents of this site towards or away from any viewpoint.

This project has been featured in the following instances:

- [Leveraging Log Probabilities in Language Models to Forecast Future Events](https://arxiv.org/abs/2501.04880v1)
- [Tangle News: Lessons from the election you could bet on](https://www.readtangle.com/otherposts/lessons-from-the-election-you-could-bet-on/)
- [Forecasting Newsletter: June 2024](https://forecasting.substack.com/p/forecasting-newsletter-june-2024)
- [Calibrations Blog: Should we let ourselves see the future?](https://www.calibrations.blog/p/should-we-let-ourselves-see-the-future)
- [Lightcone News: Accuracy and Trust](https://lightcone.news/about)
- [Valis Research: Unanswered Questions Surrounding Prediction Markets](https://valisresearch.xyz/work/unanswered-questions-surrounding-prediction-markets/index.html)
- [Human Invariant: The Future of Play Money Prediction Markets](https://www.humaninvariant.com/blog/pm-play)

I use prediction markets, mainly Manifold and Metaculus, as a personal exercise in calibration. This project grew out of an effort to see how useful they can be as information-gathering tools.

As with any statistics, this data can be used to tell many stories. I do my best to present this data in a way that is fair, accurate, and with sufficient context.

## License

The code for this project as presented in this repository is copyright under the MIT License, attached.

The contents of the live published website and database, including the explanatory descriptions, market/question links, categorizations, graphics, and visualizations are copyright under CC BY-NC-SA 4.0 Deed, attached.
