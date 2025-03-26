# What is this

This is Project Themis, a suite of tools which powers the site [Calibration City](https://calibration.city/). The purpose of this project is to perform useful analysis of prediction market calibration and accuracy with data from each platform's public API.

NOTE: This project is currently undergoing a rewrite, and not all components may be usable as-is.

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
  - There are a few Python scripts I use for development in the `scripts` folder. If you want to use these, ensure you have a recent version of Python installed.
  - When testing API responses I use `jq` for filtering and general formatting. You can get that [here](https://jqlang.org/download/).
  - A couple scripts for debugging are written with `rust-script`. Installation instructions are [here](https://rust-script.org/#installation).

## Step 1. Downloading API data to disk

In previous versions of this program, we deserialized all API responses immediately upon receiving them in order to work in a type-safe rust environment. This works great if APIs never change. Since external APIs can change unexpectedly, we have broken the download flow into two programs: a downloader and an extractor. The downloader will grab all relevant data from the platform APIs, then the extractor will deserialize that data into something we can use.

Before downloading, make sure you have enough disk space, memory, and time:

- By default the download program will download from all platforms in parallel to avoid getting bottle-necked by any one platform's API rate limit. In order to do this we first download the platform's bulk list as an index and load it into memory. If you are running in the default mode, expect to use around 6 GB of memory. If you run out of memory, you can run the platforms one at a time with the `--platform` option.
- This program will download all relevant data from each platform's API to disk. We try to avoid reading or writing any more than necessary by buffering writes and appending data where possible. Still, a large amount of disk space will be required for this data. As of February 2025 it uses around 20 GB, but this will increase over time.
- When run in parallel (default configuration), this utility takes around 8 hours to complete. It will first download the index and make a download plan. Then it will queue up batches of downloads that run asynchronously. If you interrupt the program or it runs into an error, simply restart it. It will look for an existing index file and attempt to resume the downloads automatically.

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

This command will start the database, the REST adapter, and the backup utility. These services only need to be running during the `extract` process, group building process, and site building process.

The database will run in Postgres, which will persist data in the `postgres_data` folder. You should never need to access or edit the contents of this folder. Another container handles backups, which will be placed in the `postgres_backups` folder daily.

If you ever change a setting in the `.env` file, you can re-run `just db-up` to reload the configuration and restart containers if necessary.

To manually run a backup or get the database schema:

```bash
just db-backup # run a backup and save in the postgres_backups folder
just db-schema # extract the current schema and output to stdout
```

Import the schema, roles, and some basic data by running the following SQL files:

```bash
just db-run-sql 01-schema.sql
just db-run-sql 02-postgrest.sql
just db-run-sql 03-platforms.sql
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

## Step 4. Creating and processing groups

TODO: Incomplete.

## Step 5. Generating site

The site is static and designed to be deployed behind any standard web server such as `nginx`. It could also be deployed to GitHub Pages, Cloudflare Pages, or an AWS S3 bucket.

You can view a preview of the site or build it like so:

```bash
just dev # live preview the site in a browser
just build # build the site to the site/dist directory
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

## Step 6. Downloading new markets

Over time, new markets will be added and other markets will be updated. In order to update the database with the freshest data, you can re-run the download and extract programs to load the new data.

The download program has two different arguments for resetting:

- `--reset-index` will download and add _newly-added_ markets to the database but not ones downloaded previously that have since been updated. This is a simple top-up and probably not what you want for updating a database.
- `--reset-cache` will re-download _everything,_ updating the database with 100% fresh data. Note that it will _not_ remove markets from the database if they have been removed from the platform.

Both options make a backup of the previous data files in case you want to look at past data.

To run a full refresh and import the data into the database:

```bash
just download --reset-cache
just extract
```

After the data is downloaded, you can add groups and edit data in the database as before. Then, build the site again and see the results.

# I just want the data

TODO: This isn't actually set up yet. See https://calibration.city/ for the current live site.

The production database is publicly readable via PostgREST here:

- [https://data.predictionmetrics.org](https://data.predictionmetrics.org/)

For example, to get items from various tables:

```bash
curl -sf https://data.predictionmetrics.org/platforms
curl -sf https://data.predictionmetrics.org/questions
curl -sf https://data.predictionmetrics.org/markets
```

You can find PostgREST documentation here:

- https://docs.postgrest.org/en/v12/references/api/tables_views.html
- https://docs.postgrest.org/en/v12/references/api/pagination_count.html

# Notes, news, and disclaimers

This project has been awarded the following grants:

- $3,500 as part of the [Manifold Community Fund](https://manifund.org/projects/wasabipestos-umbrella-project), an impact certificate-based funding round.
- $3,764 as part of the [EA Community Choice](https://manifund.org/projects/calibration-city), a donation matching pool.

These grants have been used for furthering development but have not influenced the contents of this site towards or away from any viewpoint.

This project has been featured in the following publications:

- [Leveraging Log Probabilities in Language Models to Forecast Future Events](https://arxiv.org/abs/2501.04880v1)
- [Forecasting Newsletter: June 2024](https://forecasting.substack.com/p/forecasting-newsletter-june-2024)

I use prediction markets, mainly Manifold and Metaculus, as a personal exercise in calibration. This project grew out of an effort to see how useful they can be as information-gathering tools.

As with any statistics, this data can be used to tell many stories. I do my best to present this data in a way that is fair, accurate, and with sufficient context.
