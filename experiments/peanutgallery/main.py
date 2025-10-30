"""
PeanutGallery HTTP API

Simple Flask API for making predictions on market questions using trained models.
"""

import os
from flask import Flask, request, jsonify
from dotenv import load_dotenv
from utils.ngram_predictor import NGramPredictor
from utils.embedding_predictor import EmbeddingPredictor

app = Flask(__name__)

# Global variables to store predictors
embedding_predictor = None
ngram_predictor = None

@app.route('/sally', methods=['POST'])
def sally():
    """
    Predict resolution and metadata based on models trained on natural language embeddings.
    Requires the market title and description (grouped as "question") or really any question text.
    """
    try:
        # Parse request
        if not request.is_json:
            return jsonify({"status": "error", "message": "Request must be JSON"}), 400

        data = request.get_json()
        if not data or 'question' not in data:
            return jsonify({"status": "error", "message": "Missing 'question' field"}), 400

        question = data['question'].strip()
        if not question:
            return jsonify({"status": "error", "message": "Question cannot be empty"}), 400

        # Check if embedding predictor is loaded
        if embedding_predictor is None:
            return jsonify({"status": "error", "message": "Embedding predictor not loaded"}), 500

        # Make predictions using the embedding predictor
        prediction_result = embedding_predictor.predict_all(question)

        # Check if prediction was successful
        if 'error' in prediction_result:
            return jsonify({"status": "error", "message": prediction_result['error']}), 500

        return jsonify({
            "status": "success",
            "results": prediction_result
        })

    except Exception as e:
        return jsonify({"status": "error", "message": str(e)}), 500

@app.route('/joe', methods=['POST'])
def joe():
    """
    Predict resolution based on title word frequency analysis (n-grams).
    Requires market title or any question text.
    """
    try:
        # Parse request
        if not request.is_json:
            return jsonify({"status": "error", "message": "Request must be JSON"}), 400

        data = request.get_json()
        if not data or 'question' not in data:
            return jsonify({"status": "error", "message": "Missing 'question' field"}), 400

        question = data['question'].strip()
        if not question:
            return jsonify({"status": "error", "message": "Question cannot be empty"}), 400

        # Check if ngram predictor is loaded
        if ngram_predictor is None:
            return jsonify({"status": "error", "message": "N-gram predictor not loaded"}), 500

        try:
            # Get prediction from ngram predictor
            prediction_result = ngram_predictor.predict_resolution(question)

            # Check if prediction was successful
            if prediction_result.get('error'):
                return jsonify({"status": "error", "message": prediction_result['error']}), 500

            # Format results to match the expected output format
            results = {
                "resolution_prediction": prediction_result['prediction'],
                "prob_resolution_0": prediction_result['prob_resolution_0'],
                "prob_resolution_1": prediction_result['prob_resolution_1'],
                "confidence": prediction_result['confidence'],
                "uncertainty": prediction_result['uncertainty'],
                "evidence_count": prediction_result['evidence_count'],
                "strongest_evidence": prediction_result.get('strongest_evidence')
            }

        except Exception as e:
            return jsonify({"status": "error", "message": f"N-gram prediction failed: {str(e)}"}), 500

        return jsonify({
            "status": "success",
            "results": results
        })

    except Exception as e:
        return jsonify({"status": "error", "message": str(e)}), 500

@app.route('/health', methods=['GET'])
def health_check():
    """Simple health check endpoint."""
    embedding_models_count = len(embedding_predictor.models) if embedding_predictor else 0
    ngram_loaded = ngram_predictor is not None
    return jsonify({
        "status": "healthy",
        "embedding_models_loaded": embedding_models_count,
        "ngram_predictor_loaded": ngram_loaded
    })

@app.route('/', methods=['GET'])
def index():
    """API information endpoint."""
    return jsonify({
        "name": "The Peanut Gallery",
        "version": "0.1.0",
        "endpoints": {
            "/": "GET - This information",
            "/health": "GET - Health check",
            "/sally": "POST - Predict resolution and metadata based on models trained on natural language embeddings",
            "/joe": "POST - Predict resolution based on title word frequency analysis (n-grams).",
        },
        "embedding_models_loaded": len(embedding_predictor.models) if embedding_predictor else 0,
        "ngram_predictor_loaded": ngram_predictor is not None
    })

def main():
    """Main function to run the Flask app."""
    # Load environment variables
    load_dotenv()

    # Load predictors on startup
    global embedding_predictor, ngram_predictor

    # Load embedding predictor
    try:
        model_dir = os.path.join(os.path.dirname(__file__), 'models', 'embeddings')
        embedding_predictor = EmbeddingPredictor(model_dir)
        if not embedding_predictor.models:
            print("Warning: No embedding models loaded. Sally endpoint predictions will fail.")
        else:
            print(f"Loaded {len(embedding_predictor.models)} embedding models successfully")
    except Exception as e:
        print(f"Warning: Could not load embedding predictor: {e}")
        embedding_predictor = None

    # Load ngram predictor
    try:
        ngram_data_path = os.path.join(os.path.dirname(__file__), 'models', 'ngram_counts.pkl')
        ngram_predictor = NGramPredictor(ngram_data_path)
        print("N-gram predictor loaded successfully")
    except Exception as e:
        print(f"Warning: Could not load N-gram predictor: {e}")
        ngram_predictor = None

    # Start Flask app
    port = int(os.environ.get('PORT', 5000))
    host = os.environ.get('HOST', '0.0.0.0')
    debug = os.environ.get('DEBUG', 'False').lower() == 'true'

    print(f"Starting PeanutGallery API on http://{host}:{port}")
    app.run(host=host, port=port, debug=debug)

if __name__ == "__main__":
    main()
