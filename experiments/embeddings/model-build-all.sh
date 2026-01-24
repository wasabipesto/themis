for target in resolution volume_usd high_volume traders_count high_traders duration_days high_duration; do
    uv run model-build.py -sp manifold -ts 0.05 -t $target
done
