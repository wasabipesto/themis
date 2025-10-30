"""
PeanutGallery HTTP API

Simple Flask API for making predictions on market questions using trained models.
"""

import os
import re
import pickle
from flask import Flask, request, jsonify
import ollama
import numpy as np
from dotenv import load_dotenv
from utils.ngram_predictor import NGramPredictor

app = Flask(__name__)

# Global variables to store loaded models and ngram predictor
loaded_models = []
ngram_predictor = None

def load_all_models(model_dir="./models"):
    """Load all models from the model directory."""
    models = []

    if not os.path.exists(model_dir):
        print(f"Warning: Model directory not found: {model_dir}")
        return models

    model_files = [f for f in os.listdir(model_dir) if f.endswith('.pkl')]
    if not model_files:
        print(f"Warning: No model files found in {model_dir}")
        return models

    for model_file in model_files:
        model_path = os.path.join(model_dir, model_file)
        try:
            with open(model_path, 'rb') as f:
                model_data = pickle.load(f)
            models.append(model_data)
            print(f"Loaded model: {model_data['model_name']}/{model_data['target_column']}")
        except Exception as e:
            print(f"Warning: Could not load {model_file}: {e}")

    return models

def generate_embeddings(prompt):
    """Generate embeddings for a given prompt."""
    try:
        response = ollama.embeddings(
            model="nomic-embed-text",
            prompt=prompt,
        )
        embedding = response.get("embedding")
        if not embedding or len(embedding) != 768:
            raise Exception(f"Invalid embedding response: {response}")
        return embedding
    except Exception as e:
        raise Exception(f"Failed to generate embeddings: {e}")


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

        # Load models if not already loaded
        if not loaded_models:
            return jsonify({"status": "error", "message": "No models available"}), 500

        # Make predictions with each model
        results = {}
        for model in loaded_models:
            try:
                # Generate embeddings from scratch
                embeddings = generate_embeddings(question)

                # Add platform indicators (Manifold only)
                platform_features = [1,0,0,0]
                all_features = np.append(embeddings, platform_features)
                all_features = all_features.reshape(1, -1)

                # Apply PCA if it was used during training
                if model.get('pca') is not None:
                    all_features = model['pca'].transform(all_features)

                # Make the prediction
                prediction = model['model'].predict(all_features)[0]

                # Save the results
                target_column = model['target_column']
                results[target_column] = float(prediction)

            except Exception as e:
                print(f"Warning: Could not predict {model['target_column']}: {e}")
                continue

        if not results:
            return jsonify({"status": "error", "message": "No predictions could be made"}), 500

        return jsonify({
            "status": "success",
            "results": results
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

        # Load models if not already loaded
        if not loaded_models:
            return jsonify({"status": "error", "message": "No models available"}), 500

        # Use ngram predictor if available
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
    return jsonify({"status": "healthy", "models_loaded": len(loaded_models)})

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
        "models_loaded": len(loaded_models)
    })

def main():
    """Main function to run the Flask app."""
    # Load environment variables
    load_dotenv()

    # Load models on startup
    global loaded_models, ngram_predictor
    model_dir = os.path.join(os.path.dirname(__file__), 'models', 'embeddings')
    loaded_models = load_all_models(model_dir)

    if not loaded_models:
        print("Warning: No models loaded. The API will start but predictions will fail.")
    else:
        print(f"Loaded {len(loaded_models)} models successfully")

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
