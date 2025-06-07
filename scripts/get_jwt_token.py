#!/usr/bin/env python3

"""
fast-tag API JWT Token Retrieval Script
Usage: python scripts/get_jwt_token.py [google|github]
"""

import sys
import time
import json
import os
import subprocess
import platform
from urllib.request import urlopen, Request
from urllib.error import URLError

# Default settings
API_BASE_URL = "http://localhost:8080"
MAX_WAIT_TIME = 300  # 5 minutes


def show_usage():
    """Show usage information."""
    print("Usage: python get_jwt_token.py [google|github]")
    print("")
    print("Examples:")
    print("  python get_jwt_token.py google   # Use Google authentication")
    print("  python get_jwt_token.py github   # Use GitHub authentication")
    print("")
    sys.exit(1)


def make_request(url, headers=None):
    """Make HTTP GET request and return JSON response."""
    try:
        req = Request(url, headers=headers or {})
        with urlopen(req) as response:
            return json.loads(response.read().decode())
    except URLError as e:
        return None
    except json.JSONDecodeError:
        return None


def open_browser(url):
    """Open URL in the default browser."""
    system = platform.system()
    if system == "Darwin":  # macOS
        subprocess.run(["open", url])
    elif system == "Linux":
        subprocess.run(["xdg-open", url])
    elif system == "Windows":
        os.startfile(url)
    else:
        print("Please copy the above URL to your browser and complete authentication")


def main():
    # Get auth provider from command line
    auth_provider = sys.argv[1] if len(sys.argv) > 1 else "google"
    
    # Validate arguments
    if auth_provider not in ["google", "github"]:
        print(f"Error: Auth provider must be 'google' or 'github'")
        show_usage()
    
    print("=== fast-tag API JWT Token Retrieval ===")
    print(f"Auth Provider: {auth_provider}")
    print(f"API URL: {API_BASE_URL}")
    print("")
    
    # 1. Get authentication URL
    print(f"1. Getting {auth_provider} authentication URL...")
    auth_response = make_request(f"{API_BASE_URL}/auth/{auth_provider}")
    
    if not auth_response:
        print("Error: Cannot connect to API server")
        print("Make sure the API server is running: cargo run -p api")
        sys.exit(1)
    
    auth_url = auth_response.get("auth_url")
    poll_token = auth_response.get("poll_token")
    
    if not auth_url or not poll_token:
        print("Error: Failed to get authentication URL")
        print(f"Response: {auth_response}")
        sys.exit(1)
    
    print("✓ Authentication URL retrieved")
    print("")
    
    # 2. Open browser for authentication
    print("2. Please authenticate in your browser:")
    print(auth_url)
    print("")
    
    print("Opening browser automatically...")
    open_browser(auth_url)
    
    print("")
    print("This script will continue automatically after authentication...")
    
    # 3. Poll for token
    print("")
    print("3. Waiting for authentication completion...", end="", flush=True)
    start_time = time.time()
    
    while True:
        elapsed_time = time.time() - start_time
        
        if elapsed_time > MAX_WAIT_TIME:
            print("")
            print(f"Error: Timeout after {MAX_WAIT_TIME} seconds")
            print("Please retry authentication")
            sys.exit(1)
        
        print(".", end="", flush=True)
        poll_response = make_request(f"{API_BASE_URL}/auth/poll/{poll_token}")
        
        if not poll_response:
            time.sleep(2)
            continue
        
        status = poll_response.get("status")
        
        if status == "completed":
            jwt_token = poll_response.get("jwt")
            print("")
            print("✓ Authentication completed!")
            break
        elif status == "pending":
            time.sleep(2)
        else:
            print("")
            print("Error: Authentication failed")
            print(f"Response: {poll_response}")
            sys.exit(1)
    
    # 4. Display and save token
    print("")
    print("=== JWT Token Retrieved ===")
    print(f"Token: {jwt_token}")
    print("")
    
    # Save to environment file
    token_file = ".jwt_token"
    with open(token_file, "w") as f:
        f.write(f'JWT_TOKEN="{jwt_token}"\n')
    print(f"✓ Token saved to {token_file}")
    print("")
    
    # 5. Test the token
    print("4. Testing token...")
    headers = {"Authorization": f"Bearer {jwt_token}"}
    user_info = make_request(f"{API_BASE_URL}/me", headers=headers)
    
    if user_info:
        print("✓ Token is valid")
        print(f"User info: {json.dumps(user_info, indent=2)}")
    else:
        print("⚠️  Token validation failed")
    
    print("")
    print("=== Usage Examples ===")
    print("# Load token as environment variable")
    print("source .jwt_token")
    print("")
    print("# Call API")
    print(f'curl -H "Authorization: Bearer $JWT_TOKEN" {API_BASE_URL}/projects')
    print("")
    print("# Or use directly")
    print(f'export JWT_TOKEN="{jwt_token}"')
    print(f'curl -H "Authorization: Bearer $JWT_TOKEN" {API_BASE_URL}/projects')


if __name__ == "__main__":
    main()