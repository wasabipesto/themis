"""
PeanutGallery HTTP API

Simple Flask API for making predictions on market questions using trained models.
"""

import os
from flask import Flask, request, jsonify
from dotenv import load_dotenv
from utils.position_predictor import *
from utils.ngram_predictor import NGramPredictor
from utils.embedding_predictor import EmbeddingPredictor

app = Flask(__name__)

# Global variables to store predictors
position_predictor = None
embedding_predictor = None
ngram_predictor = None

@app.route('/charlie', methods=['POST'])
def charlie():
    """
    Predict resolution based on Manifold user positions and profit.
    Requires a manifold slug for lookup.
    """
    try:
        # Parse request
        if not request.is_json:
            return jsonify({"status": "error", "message": "Request must be JSON"}), 400

        data = request.get_json()
        if not data or 'slug' not in data:
            return jsonify({"status": "error", "message": "Missing 'slug' field"}), 400

        slug = data['slug'].strip()
        if not slug:
            return jsonify({"status": "error", "message": "Slug cannot be empty"}), 400

        # Check if predictor is loaded
        if position_predictor is None:
            return jsonify({"status": "error", "message": "Position predictor not loaded"}), 500

        try:
            # Get prediction from position predictor
            results = position_predictor.predict_outcome(slug)

        except Exception as e:
            return jsonify({"status": "error", "message": f"Position prediction failed: {str(e)}"}), 500

        return jsonify({
            "status": "success",
            "results": results
        })

    except Exception as e:
        return jsonify({"status": "error", "message": str(e)}), 500

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

        # Check if predictor is loaded
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

        # Check if predictor is loaded
        if ngram_predictor is None:
            return jsonify({"status": "error", "message": "N-gram predictor not loaded"}), 500

        try:
            # Get prediction from ngram predictor
            results = ngram_predictor.predict_resolution(question)

            # Check if prediction was successful
            if results.get('error'):
                return jsonify({"status": "error", "message": results['error']}), 500

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
    position_predictor_stats = position_predictor.get_cache_stats() if position_predictor else None
    embedding_models_count = len(embedding_predictor.models) if embedding_predictor else 0
    ngram_stats = ngram_predictor.get_stats() if ngram_predictor else None
    return jsonify({
        "status": "healthy",
        "position_predictor_stats": position_predictor_stats,
        "embedding_models_loaded": embedding_models_count,
        "ngram_predictor_stats": ngram_stats
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
            "/charlie": "POST - Predict resolution based on Manifold user positions and profit",
            "/sally": "POST - Predict resolution and metadata based on models trained on natural language embeddings",
            "/joe": "POST - Predict resolution based on title word frequency analysis (n-grams)",
        },
        "embedding_models_loaded": len(embedding_predictor.models) if embedding_predictor else 0,
        "ngram_predictor_loaded": ngram_predictor is not None
    })

def main():
    """Main function to run the Flask app."""
    # Load environment variables
    load_dotenv()

    # Load predictors on startup
    global position_predictor, embedding_predictor, ngram_predictor

    # Load position predictor
    try:
        cache_dir = os.path.join(os.path.dirname(__file__), 'cache')
        position_predictor = PositionPredictor(cache_dir)
        print(f"Positions predictor loaded successfully")
    except Exception as e:
        print(f"Warning: Could not load positions predictor: {e}")
        position_predictor = None

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
