# The Peanut Gallery

A fun, lighthearted web application and API for getting advice from the "peanut gallery" - a group of characters who give their thoughts on prediction markets using different ML approaches.

## Usage

### Web Interface

Start the Flask app and navigate to http://localhost:5000 to use the interactive webpage:

```bash
uv run main.py
```

The webpage features:
- **Omnibar**: Paste a Manifold market URL or type any question
- **Character Gallery**: Three characters with different prediction approaches:
  - **Charlie**: Analyzes Manifold user positions and profits (requires market URL)
  - **Sally**: Uses natural language embeddings to understand question semantics
  - **Linus**: Analyzes word patterns and n-grams in question text

When you provide a Manifold URL, the app fetches market info and shows current probability. All characters will analyze the market and share their predictions with supporting details.

### API Endpoints

Make direct API requests:

```bash
# Charlie - Position-based predictions (requires Manifold slug)
curl -sfX POST http://localhost:5000/api/charlie \
  -H "Content-Type: application/json" \
  -d '{"slug": "will-we-get-agi-before-2030"}' \
  | jq

# Sally - Embedding-based predictions
curl -sfX POST http://localhost:5000/api/sally \
  -H "Content-Type: application/json" \
  -d '{"question": "Will AI achieve AGI by 2030?"}' \
  | jq

# Linus - N-gram pattern predictions
curl -sfX POST http://localhost:5000/api/linus \
  -H "Content-Type: application/json" \
  -d '{"question": "Will AI achieve AGI by 2030?"}' \
  | jq
```

## Features

- **Neobrutalist Design**: Bold, cartoonish interface inspired by Peanuts comics
- **Responsive Layout**: Works on desktop and mobile devices
- **Real-time Market Data**: Fetches current Manifold market information
- **Character Personalities**: Each predictor has a unique voice and approach
- **Error Handling**: Graceful handling of API errors and invalid inputs
