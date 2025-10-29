for target in score high_score high_volume high_traders; do
    uv run predictive-model.py -sp manifold -t $target
done
