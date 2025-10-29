for target in resolution score high_score high_volume high_traders; do
    uv run model-build.py -sp manifold -t $target
done
