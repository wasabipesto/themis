import os
import json
import time
import pickle
import requests
import argparse
from typing import Dict, List, Optional, Tuple, Any
from dataclasses import dataclass
from datetime import datetime, timedelta


@dataclass
class UserData:
    """Cached user data from Manifold API."""
    id: str
    username: str
    name: str
    balance: float
    total_deposits: float
    created_time: int
    is_bot: bool = False
    profit: Optional[float] = None


@dataclass
class Position:
    """User position data for a market."""
    user_id: str
    shares: Dict[str, float]  # outcome -> shares
    invested: float
    profit: float
    last_bet_time: int


@dataclass
class PredictionResult:
    """Result of position-based prediction."""
    predicted_outcome: float  # 0-1 probability
    uncertainty: float  # 0-1 uncertainty measure
    top_weighted_user: Optional[Dict[str, Any]]  # Most influential user
    total_positions: int
    total_weighted_value: float
    methodology: str = "position_weighted"


class PositionPredictor:
    def __init__(self, cache_dir: str = "cache", cache_expiry_hours: int = 24):
        """
        Initialize the Position predictor.

        Args:
            cache_dir: Directory to store cached user data
            cache_expiry_hours: Hours before cache expires (default 24)
        """
        self.cache_dir = cache_dir
        self.cache_expiry_hours = cache_expiry_hours
        self.cache_file = os.path.join(cache_dir, "manifold_users.pkl")
        self.users_cache: Dict[str, UserData] = {}

        # Ensure cache directory exists
        os.makedirs(cache_dir, exist_ok=True)

        # Load cached user data if available and not expired
        self._load_user_cache()

        # Base API URL
        self.api_base = "https://api.manifold.markets/v0"

    def _load_user_cache(self) -> None:
        """Load user data from cache if it exists and is not expired."""
        if not os.path.exists(self.cache_file):
            return

        try:
            cache_stat = os.stat(self.cache_file)
            cache_age = time.time() - cache_stat.st_mtime
            max_age = self.cache_expiry_hours * 3600  # Convert to seconds

            if cache_age > max_age:
                print(f"Cache is {cache_age/3600:.1f} hours old, refreshing...")
                return

            with open(self.cache_file, 'rb') as f:
                cache_data = pickle.load(f)
                self.users_cache = cache_data.get('users', {})
                cache_time = cache_data.get('timestamp', 0)

            print(f"Loaded {len(self.users_cache)} users from cache "
                  f"(created {(time.time() - cache_time)/3600:.1f} hours ago)")

        except Exception as e:
            print(f"Warning: Could not load user cache: {e}")
            self.users_cache = {}

    def _save_user_cache(self) -> None:
        """Save user data to cache."""
        try:
            cache_data = {
                'users': self.users_cache,
                'timestamp': time.time()
            }
            with open(self.cache_file, 'wb') as f:
                pickle.dump(cache_data, f)
            print(f"Saved {len(self.users_cache)} users to cache")
        except Exception as e:
            print(f"Warning: Could not save user cache: {e}")

    def _fetch_all_users(self) -> None:
        """Fetch all users from Manifold API and cache them."""
        print("Fetching all users from Manifold API...")

        all_users = {}
        limit = 1000
        before = None
        page = 1

        while True:
            url = f"{self.api_base}/users"
            params = {'limit': limit}
            if before:
                params['before'] = before

            try:
                response = requests.get(url, params=params, timeout=30)
                response.raise_for_status()
                users = response.json()

                if not users:
                    break

                print(f"Page {page}: fetched {len(users)} users")

                for user_data in users:
                    user = UserData(
                        id=user_data['id'],
                        username=user_data['username'],
                        name=user_data['name'],
                        balance=user_data.get('balance', 0),
                        total_deposits=user_data.get('totalDeposits', 0),
                        created_time=user_data['createdTime'],
                        is_bot=user_data.get('isBot', False),
                        profit=user_data.get('totalPnLCached')
                    )
                    all_users[user.id] = user

                # Set up for next page
                before = users[-1]['id']
                page += 1

                # Rate limiting - be nice to the API
                time.sleep(0.1)

            except requests.exceptions.RequestException as e:
                print(f"Error fetching users: {e}")
                break

        self.users_cache = all_users
        self._save_user_cache()
        print(f"Finished fetching {len(all_users)} total users")

    def _ensure_users_loaded(self) -> None:
        """Ensure user data is loaded, fetching if necessary."""
        if not self.users_cache:
            self._fetch_all_users()

    def _market_slug_to_id(self, market_slug: str) -> int:
        """Convert a market slug to its ID."""
        url = f"{self.api_base}/slug/{market_slug}"
        response = requests.get(url, timeout=30)
        response.raise_for_status()
        market_data = response.json()
        return market_data['id']

    def _fetch_market_positions(self, market_slug: str) -> List[Position]:
        """
        Fetch positions for a market by slug.

        Args:
            market_slug: The market slug to fetch positions for

        Returns:
            List of Position objects
        """
        market_id = self._market_slug_to_id(market_slug)
        url = f"{self.api_base}/market/{market_id}/positions"
        params = {
            'order': 'shares',
            'top': 1000
        }

        try:
            response = requests.get(url, params=params, timeout=30)
            response.raise_for_status()
            positions_data = response.json()

            positions = []
            for pos_data in positions_data:
                # Extract share information
                shares = pos_data.get('totalShares', {})

                position = Position(
                    user_id=pos_data['userId'],
                    shares=shares,
                    invested=pos_data.get('invested', 0),
                    profit=pos_data.get('profit', 0),
                    last_bet_time=pos_data.get('lastBetTime', 0)
                )
                positions.append(position)

            return positions

        except requests.exceptions.RequestException as e:
            raise Exception(f"Failed to fetch market positions: {e}")

    def _calculate_user_weight(self, user: UserData, position: Position,
                             weighting_config: Optional[Dict[str, float]] = None) -> float:
        """
        Calculate the weight for a user's position.

        Args:
            user: UserData for the user
            position: Position data for this market
            weighting_config: Optional weighting configuration

        Returns:
            Weight value for this user's position
        """
        if weighting_config is None:
            weighting_config = {
                'balance_weight': 0.5,
                'shares_weight': 1.0,
                'age_weight': 1.0,
                'profit_weight': 1.0,
            }

        # Base weight: shares * balance
        total_shares = sum(abs(shares) for shares in position.shares.values())
        base_weight = total_shares * user.balance

        # Additional weighting factors
        weight_multiplier = 1.0

        # Account age weighting (older accounts get slight boost)
        if weighting_config['age_weight'] > 0:
            account_age_days = (time.time() * 1000 - user.created_time) / (1000 * 24 * 3600)
            age_factor = min(account_age_days / 365, 2.0)  # Cap at 2x for 1+ year accounts
            weight_multiplier += weighting_config['age_weight'] * age_factor

        # Profit weighting (more successful traders get boost)
        if weighting_config['profit_weight'] > 0 and user.profit is not None:
            profit_factor = max(0, min(user.profit / 10000, 1.0))  # Normalize profit
            weight_multiplier += weighting_config['profit_weight'] * profit_factor

        return base_weight * weight_multiplier

    def predict_outcome(self, market_slug: str,
                       weighting_config: Optional[Dict[str, float]] = None,
                       verbose: bool = False) -> PredictionResult:
        """
        Predict the outcome of a market based on user positions.

        Args:
            market_slug: The market slug to predict
            weighting_config: Optional configuration for position weighting
            verbose: Whether to show detailed information

        Returns:
            PredictionResult with prediction and metadata
        """
        self._ensure_users_loaded()

        if verbose:
            print(f"Predicting outcome for market: {market_slug}")

        # Fetch market positions
        positions = self._fetch_market_positions(market_slug)

        if not positions:
            return PredictionResult(
                predicted_outcome=0.5,  # Default to 50/50
                uncertainty=1.0,  # Maximum uncertainty
                top_weighted_user=None,
                total_positions=0,
                total_weighted_value=0.0
            )

        # Calculate weighted outcomes
        weighted_yes = 0.0
        weighted_no = 0.0
        total_weight = 0.0
        user_weights = []

        for position in positions:
            user = self.users_cache.get(position.user_id)
            if not user:
                continue  # Skip users not in cache

            # Skip bot accounts for cleaner predictions
            if user.is_bot:
                continue

            weight = self._calculate_user_weight(user, position, weighting_config)

            # Get YES and NO shares
            yes_shares = position.shares.get('YES', 0)
            no_shares = position.shares.get('NO', 0)

            # Weight the outcomes by shares held
            weighted_yes += weight * yes_shares
            weighted_no += weight * abs(no_shares)  # NO shares are typically negative
            total_weight += weight

            user_weights.append({
                'user': user,
                'position': position,
                'weight': weight,
                'yes_shares': yes_shares,
                'no_shares': no_shares
            })

        if total_weight == 0:
            return PredictionResult(
                predicted_outcome=0.5,
                uncertainty=1.0,
                top_weighted_user=None,
                total_positions=len(positions),
                total_weighted_value=0.0
            )

        # Calculate predicted probability
        total_weighted_outcome = weighted_yes + weighted_no
        if total_weighted_outcome > 0:
            predicted_prob = weighted_yes / total_weighted_outcome
        else:
            predicted_prob = 0.5

        # Calculate uncertainty
        # Higher uncertainty when:
        # 1. Fewer positions
        # 2. More balanced between YES/NO
        # 3. Lower total weight (less confident users)

        position_uncertainty = max(0, 1 - len(positions) / 100)  # Less uncertain with more positions
        balance_uncertainty = 1 - abs(predicted_prob - 0.5) * 2  # More uncertain when close to 50/50
        weight_uncertainty = max(0, 1 - total_weight / 1000000)  # Less uncertain with higher total weight

        uncertainty = (position_uncertainty + balance_uncertainty + weight_uncertainty) / 3
        uncertainty = max(0.0, min(1.0, uncertainty))  # Clamp to [0, 1]

        # Find top weighted user
        top_user = None
        if user_weights:
            top_weighted = max(user_weights, key=lambda x: x['weight'])
            top_user = {
                'user_id': top_weighted['user'].id,
                'username': top_weighted['user'].username,
                'name': top_weighted['user'].name,
                'weight': top_weighted['weight'],
                'balance': top_weighted['user'].balance,
                'yes_shares': top_weighted['yes_shares'],
                'no_shares': top_weighted['no_shares']
            }

        result = PredictionResult(
            predicted_outcome=predicted_prob,
            uncertainty=uncertainty,
            top_weighted_user=top_user,
            total_positions=len(positions),
            total_weighted_value=total_weight
        )

        if verbose:
            print(f"Processed {len(positions)} positions")
            print(f"Weighted YES: {weighted_yes:,.0f}")
            print(f"Weighted NO: {weighted_no:,.0f}")
            print(f"Total weight: {total_weight:,.0f}")
            print(f"Predicted probability: {predicted_prob:.3f}")
            print(f"Uncertainty: {uncertainty:.3f}")
            if top_user:
                print(f"Top weighted user: {top_user['username']} (weight: {top_user['weight']:,.0f})")

        return result

    def refresh_user_cache(self) -> None:
        """Force refresh of user cache."""
        print("Force refreshing user cache...")
        self._fetch_all_users()

    def get_cache_stats(self) -> Dict[str, Any]:
        """Get statistics about the cached user data."""
        if not os.path.exists(self.cache_file):
            return {'cache_exists': False}


        cache_stat = os.stat(self.cache_file)
        cache_age = time.time() - cache_stat.st_mtime

        return {
            'cache_exists': True,
            'users_cached': len(self.users_cache),
            'cache_age_hours': cache_age / 3600,
            'cache_size_mb': cache_stat.st_size / (1024 * 1024),
            'is_expired': cache_age > (self.cache_expiry_hours * 3600)
        }


def main():
    parser = argparse.ArgumentParser(description='Predict market outcomes based on user positions')
    parser.add_argument('market_slug', nargs='?', help='Market slug to predict outcome for')
    parser.add_argument('--cache-dir', default='cache', help='Directory for user cache')
    parser.add_argument('--refresh-cache', action='store_true',
                       help='Force refresh of user cache')
    parser.add_argument('--cache-stats', action='store_true',
                       help='Show cache statistics')
    parser.add_argument('--verbose', '-v', action='store_true',
                       help='Show detailed analysis')
    parser.add_argument('--balance-weight', type=float, default=0.5,
                       help='Weight for user balance in calculations')
    parser.add_argument('--age-weight', type=float, default=1.0,
                       help='Weight for account age in calculations')
    parser.add_argument('--profit-weight', type=float, default=1.0,
                       help='Weight for user profit in calculations')

    args = parser.parse_args()

    try:
        predictor = PositionPredictor(cache_dir=args.cache_dir)

        # Show cache stats if requested
        if args.cache_stats:
            stats = predictor.get_cache_stats()
            print("=== Cache Statistics ===")
            if stats['cache_exists']:
                print(f"Users cached: {stats['users_cached']:,}")
                print(f"Cache age: {stats['cache_age_hours']:.1f} hours")
                print(f"Cache size: {stats['cache_size_mb']:.1f} MB")
                print(f"Cache expired: {stats['is_expired']}")
            else:
                print("No cache file exists")
            print()

        # Refresh cache if requested
        if args.refresh_cache:
            predictor.refresh_user_cache()
            return

        # Require market slug for prediction
        if not args.market_slug:
            if not args.cache_stats:
                parser.error("Market slug is required for prediction (or use --cache-stats/--refresh-cache)")
            return

        # Set up weighting configuration
        weighting_config = {
            'balance_weight': args.balance_weight,
            'shares_weight': 1.0,  # Always weight by shares
            'age_weight': args.age_weight,
            'profit_weight': args.profit_weight
        }

        # Make prediction
        result = predictor.predict_outcome(args.market_slug, weighting_config, args.verbose)

        print(f"\nMarket: {args.market_slug}")
        print(f"Predicted Outcome: {result.predicted_outcome:.3f}")
        print(f"Uncertainty: {result.uncertainty:.3f}")
        print(f"Total Positions: {result.total_positions}")
        print(f"Total Weighted Value: {result.total_weighted_value:,.0f}")

        if result.top_weighted_user:
            user = result.top_weighted_user
            print(f"Top Weighted User: {user['username']} ({user['name']})")
            print(f"  Weight: {user['weight']:,.0f}")
            print(f"  Balance: {user['balance']:,.0f}")
            print(f"  YES Shares: {user['yes_shares']:,.0f}")
            print(f"  NO Shares: {user['no_shares']:,.0f}")

    except Exception as e:
        print(f"Error: {e}")


if __name__ == "__main__":
    main()
