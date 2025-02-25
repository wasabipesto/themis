import jwt
import os
from datetime import datetime, timedelta
from dotenv import load_dotenv

# Load environment variables from .env file
load_dotenv()

# Get JWT secret from environment variable
jwt_secret = os.environ.get('PGRST_JWT_SECRET')

if not jwt_secret:
    raise ValueError("PGRST_JWT_SECRET environment variable must be set")

payload = {
    "role": "admin",
    "exp": datetime.now() + timedelta(days=30)
}

token = jwt.encode(payload, jwt_secret, algorithm="HS256")
print(token)
