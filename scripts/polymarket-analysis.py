import json
from collections import Counter
from decimal import Decimal
from datetime import datetime
import pytz

def parse_iso(date_str):
    return datetime.fromisoformat(date_str.replace('Z', '+00:00'))

def analyze_markets(jsonl_file):
    token_counts = []
    token_outcomes = set()
    yes_first_count = 0
    total_markets = 0
    winner_counts = []
    active_closed_stats = Counter()

    # New tracking variables
    end_date_present = 0
    tick_sizes = set()
    all_tags = Counter()
    price_sum_issues = 0

    # New tracking variables
    yes_no_markets = 0
    yes_first_in_yes_no = 0
    zero_winner_not_closed = 0
    zero_winner_total = 0
    two_winner_ids = []
    end_date_before_last_price = 0
    utc = pytz.UTC

    with open(jsonl_file, 'r') as f:
        for line in f:
            total_markets += 1
            data = json.loads(line)
            market = data['market']
            market_id = data['id']
            tokens = market['tokens']

            # Original analysis
            tokens = market['tokens']
            token_counts.append(len(tokens))
            token_outcomes.update(token['outcome'] for token in tokens)

            if tokens and tokens[0]['outcome'] == 'Yes':
                yes_first_count += 1

            winners = sum(1 for token in tokens if token.get('winner', False))
            winner_counts.append(winners)

            status = (market.get('active', False), market.get('closed', False))
            active_closed_stats[status] += 1

            # New analysis
            if 'end_date_iso' in market:
                end_date_present += 1

            if 'minimum_tick_size' in market:
                tick_sizes.add(market['minimum_tick_size'])

            if 'tags' in market:
                all_tags.update(market['tags'])

            # Check if token prices sum to 1 (using Decimal for precise arithmetic)
            token_prices_sum = sum(Decimal(str(token['price'])) for token in tokens)
            if abs(token_prices_sum - Decimal('1')) > Decimal('0.0001'):  # Allow small floating point differences
                price_sum_issues += 1

            # Yes/No market analysis
            if len(tokens) == 2 and {t['outcome'] for t in tokens} == {'Yes', 'No'}:
                yes_no_markets += 1
                if tokens[0]['outcome'] == 'Yes':
                    yes_first_in_yes_no += 1

            # Zero winners analysis
            winners = sum(1 for token in tokens if token.get('winner', False))
            if winners == 0:
                zero_winner_total += 1
                if not market.get('closed', False):
                    zero_winner_not_closed += 1
            elif winners == 2:
                two_winner_ids.append(market_id)

            # Timestamp analysis
            if 'end_date_iso' in market and market['end_date_iso'] and 'prices_history' in data and data['prices_history']:
                end_date = parse_iso(market['end_date_iso'])
                last_price_time = datetime.fromtimestamp(data['prices_history'][-1]['t'], tz=utc)
                if end_date < last_price_time:
                    end_date_before_last_price += 1


    print(f"Token count stats:")
    print(f"Min tokens: {min(token_counts)}")
    print(f"Max tokens: {max(token_counts)}")
    #print(f"\nUnique token outcomes: {token_outcomes}")
    print(f"\nNumber of unique token outcomes: {len(token_outcomes)}")
    print(f"\nYes appears first: {yes_first_count}/{total_markets} ({yes_first_count/total_markets*100:.1f}%)")
    print(f"\nWinner count distribution: {Counter(winner_counts)}")

    print("\nActive/Closed combinations:")
    for (active, closed), count in active_closed_stats.items():
        print(f"Active: {active}, Closed: {closed}: {count} markets ({count/total_markets*100:.1f}%)")

    print(f"end_date_iso present in {end_date_present}/{total_markets} markets ({end_date_present/total_markets*100:.1f}%)")
    print(f"\nUnique minimum_tick_size values: {sorted(tick_sizes)}")

    print("\nTop 10 most common tags:")
    for tag, count in all_tags.most_common(10):
        print(f"{tag}: {count} occurrences ({count/total_markets*100:.1f}%)")

    print(f"\nMarkets where token prices don't sum to 1: {price_sum_issues}/{total_markets} ({price_sum_issues/total_markets*100:.1f}%)")

    print(f"Yes/No markets with Yes first: {yes_first_in_yes_no}/{yes_no_markets} "
            f"({yes_first_in_yes_no/yes_no_markets*100:.1f}% of Yes/No markets)")

    print(f"\nZero winner markets not closed: {zero_winner_not_closed}/{zero_winner_total} "
            f"({zero_winner_not_closed/zero_winner_total*100:.1f}% of zero winner markets)")

    print(f"\nMarkets with exactly 2 winners ({len(two_winner_ids)}):")
    for market_id in two_winner_ids:
        print(f"- {market_id}")

    print(f"\nMarkets where end_date is before last price: {end_date_before_last_price} markets")

# Usage:
analyze_markets('cache/polymarket-data.jsonl')
