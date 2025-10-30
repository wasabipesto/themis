import os
import pickle
import numpy as np
import ollama


class EmbeddingPredictor:
    def __init__(self, model_dir="./models/embeddings"):
        """
        Initialize the Embedding predictor.

        Args:
            model_dir: Path to the directory containing trained embedding models
        """
        self.model_dir = model_dir
        self.models = self.load_all_models()

    def load_all_models(self):
        """Load all models from the model directory."""
        models = []

        if not os.path.exists(self.model_dir):
            print(f"Warning: Model directory not found: {self.model_dir}")
            return models

        model_files = [f for f in os.listdir(self.model_dir) if f.endswith('.pkl')]
        if not model_files:
            print(f"Warning: No model files found in {self.model_dir}")
            return models

        for model_file in model_files:
            model_path = os.path.join(self.model_dir, model_file)
            try:
                with open(model_path, 'rb') as f:
                    model_data = pickle.load(f)
                models.append(model_data)
                print(f"Loaded model: {model_data['model_name']}/{model_data['target_column']}")
            except Exception as e:
                print(f"Warning: Could not load {model_file}: {e}")

        return models

    def generate_embeddings(self, prompt):
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

    def predict_all(self, question):
        """
        Predict resolution and metadata for all loaded models based on the question.

        Args:
            question: The question text to make predictions for

        Returns:
            dict: Prediction results for each target column
        """
        if not question or not question.strip():
            return {
                'error': 'Question cannot be empty'
            }

        if not self.models:
            return {
                'error': 'No models available'
            }

        results = {}

        try:
            # Generate embeddings from scratch
            embeddings = self.generate_embeddings(question)
        except Exception as e:
            return {
                'error': f'Failed to generate embeddings: {str(e)}'
            }

        # Make predictions with each model
        for model in self.models:
            try:
                # Add platform indicators (Manifold only)
                platform_features = [1, 0, 0, 0]
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
            return {
                'error': 'No predictions could be made'
            }

        return results

    def predict_single(self, question, target_column):
        """
        Predict a specific target column for the given question.

        Args:
            question: The question text to make predictions for
            target_column: The specific target column to predict

        Returns:
            dict: Prediction result for the specified target column
        """
        # Find the model for the specified target column
        target_model = None
        for model in self.models:
            if model['target_column'] == target_column:
                target_model = model
                break

        if target_model is None:
            return {
                'error': f'No model found for target column: {target_column}'
            }

        if not question or not question.strip():
            return {
                'error': 'Question cannot be empty'
            }

        try:
            # Generate embeddings from scratch
            embeddings = self.generate_embeddings(question)

            # Add platform indicators (Manifold only)
            platform_features = [1, 0, 0, 0]
            all_features = np.append(embeddings, platform_features)
            all_features = all_features.reshape(1, -1)

            # Apply PCA if it was used during training
            if target_model.get('pca') is not None:
                all_features = target_model['pca'].transform(all_features)

            # Make the prediction
            prediction = target_model['model'].predict(all_features)[0]

            return {
                target_column: float(prediction)
            }

        except Exception as e:
            return {
                'error': f'Prediction failed for {target_column}: {str(e)}'
            }

    def get_model_info(self):
        """
        Get information about all loaded models.

        Returns:
            list: Information about each loaded model
        """
        info = []
        for model in self.models:
            info.append({
                'model_name': model.get('model_name', 'Unknown'),
                'target_column': model.get('target_column', 'Unknown'),
                'has_pca': model.get('pca') is not None,
                'feature_count': len(model.get('feature_names', [])) if 'feature_names' in model else 'Unknown'
            })
        return info

    def get_available_targets(self):
        """
        Get list of all available target columns.

        Returns:
            list: List of target column names
        """
        return [model['target_column'] for model in self.models]


def main():
    import argparse
    parser = argparse.ArgumentParser(description='Predict market outcomes using embedding models')
    parser.add_argument('question', help='Question to predict outcomes for')
    parser.add_argument('--model-dir', default='./models/embeddings',
                       help='Path to models directory')
    parser.add_argument('--target', help='Specific target column to predict (optional)')
    parser.add_argument('--info', action='store_true',
                       help='Show information about loaded models')

    args = parser.parse_args()

    try:
        predictor = EmbeddingPredictor(args.model_dir)

        if args.info:
            print("Loaded Models:")
            for info in predictor.get_model_info():
                print(f"  {info['model_name']}/{info['target_column']} "
                      f"(PCA: {info['has_pca']}, Features: {info['feature_count']})")
            print(f"\nAvailable targets: {predictor.get_available_targets()}")
            return

        if args.target:
            result = predictor.predict_single(args.question, args.target)
        else:
            result = predictor.predict_all(args.question)

        if 'error' in result:
            print(f"Error: {result['error']}")
            return

        print(f"Question: {args.question}")
        print("Predictions:")
        for target, prediction in result.items():
            print(f"  {target}: {prediction:.6f}")

    except Exception as e:
        print(f"Unexpected error: {e}")


if __name__ == "__main__":
    main()
