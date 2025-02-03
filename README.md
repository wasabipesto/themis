# What is this

This is Project Themis, which powers the site [Calibration City](https://calibration.city/). The purpose of this project is to perform useful analysis of prediction market calibration and accuracy with data from each platform’s public API.

This project is currently undergoing a rewrite, and not all components may be usable as-is.

# How to run this yourself

## Step 0. Install dependencies

Clone this repository and enter it:

```bash
git clone git@github.com:wasabipesto/themis.git
cd themis
```

You will need the rust toolchain to run the downloader and extractor:

```bash
curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh
```

## Step 1. Downloading API data to disk

In previous versions of this program, we deserialized all API responses immediately upon receiving them in order to work in a type-safe rust environment. This works great if APIs never change. Since external APIs can change unexpectedly, we have broken the download flow into two programs: a downloader and an extractor. The downloader will grab all relevant data from the platform APIs, then the extractor will deserialize that data into something we can use.

Before downloading, make sure you have enough disk space, memory, and time:

- By default the download program will download from all platforms in parallel to avoid getting bottle-necked by any one platform’s API rate limit. In order to do this we first download the platform’s bulk list as an index and load it into memory. If you are running in the default mode, expect to use around 6 GB of memory. If you run out of memory, you can run the platforms one at a time with the `--platform` option.
- This program will download all relevant data from each platform’s API to disk. We try to avoid reading or writing any more than necessary by buffering writes and appending data where possible. Still, a large amount of disk space will be required for this data. As of February 2025 it will use around 20 GB, but this will increase over time.
- When run in parallel (default configuration), this utility takes around 6 hours to complete. It will first download the index and make a download plan. Then it will queue up batches of downloads that run asynchronously. If you interrupt the program or it runs into an error, simply restart it. It will look for an existing index file and attempt to resume the downloads.

To run the downloader:

```bash
cd download
cargo run -- --help # for options
cargo run -r # run with default settings
```

## Step 2. Setting up the database

While the downloader is running, set up the database.

TODO:

- Docker compose
- Loading schema
- Setting environment variables

## Step 3. Importing data from the cache into the database

Once everything has been downloaded from the platform APIs, we can extract and import that data into the database.

```bash
cd ../extract
cargo run -- --help # for options
cargo run -r # run with default settings
```

## Step 4. Creating and processing groups

TODO

## Step 5. Generating site

TODO

## Step 6. Downloading new markets

Over time, new markets will be added and other markets will be updated. In order to update the database with the freshest data, you can re-run the download and extract programs to load the new data.

The download program has two different arguments for resetting:

- `--reset-index` will download and add *newly-added* markets to the database but not ones downloaded previously that have since been updated. This is a simple top-up and probably not what you want for updating a database.
- `--reset-cache` will re-download *everything,* updating the database with 100% fresh data. Note that it will *not* remove markets from the database if they have been removed from the platform.

Both options make a backup of the previous data files in case you want to look at past data.

To run a full refresh and import the data into the database:

```bash
cd ../download
cargo run -r -- --reset-cache
cd ../extract
cargo run -r
```

After the data is downloaded, you can add groups and edit data in the database as before. Then, build the site again and see the results.

# I just want the data

The production database is publicly readable via PostgREST here:

- [https://data.todo.com](https://data.todo.com/)

For example, to get items from various databases:

```bash
curl https://data.todo.com
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
