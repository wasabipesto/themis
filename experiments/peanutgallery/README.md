# The Peanut Gallery

A simple Flask HTTP API for making predictions on market questions using trained models from the embeddings experiment.

Make a request:

```bash
curl -sfX POST http://localhost:5000/sally \
  -H "Content-Type: application/json" \
  -d '{"question": "Will AI achieve AGI by 2030?"}' \
  | jq
```
