#!/usr/bin/env python3

"""
fast-tag API Testing Script
Usage: python scripts/test_storage_api.py
"""

import os
import sys
import json
import requests
from urllib.parse import quote

API_BASE_URL = "http://localhost:8080"
TOKEN_FILE = ".jwt_token"

def load_jwt_token():
    """Load JWT token from file."""
    if not os.path.exists(TOKEN_FILE):
        print(f"Error: JWT token file not found. Run python scripts/get_jwt_token.py first")
        sys.exit(1)
    
    with open(TOKEN_FILE, 'r') as f:
        content = f.read().strip()
        # Parse different token file formats
        if content.startswith("export JWT_TOKEN="):
            token = content.replace("export JWT_TOKEN=", "").strip('"\'')
            return token
        elif content.startswith("JWT_TOKEN="):
            token = content.replace("JWT_TOKEN=", "").strip('"\'')
            return token
        else:
            print("Error: Invalid token file format")
            sys.exit(1)

def print_json_response(response):
    """Pretty print JSON response."""
    try:
        data = response.json()
        print(json.dumps(data, indent=2))
    except json.JSONDecodeError:
        print(response.text)

def test_health_check():
    """Test 1: Health check."""
    print("1. Testing health endpoint...")
    response = requests.get(f"{API_BASE_URL}/health")
    print_json_response(response)
    print()

def test_user_info(headers):
    """Test 2: User info."""
    print("2. Testing user info...")
    response = requests.get(f"{API_BASE_URL}/me", headers=headers)
    print_json_response(response)
    print()

def test_projects_list(headers):
    """Test 3: List projects."""
    print("3. Testing projects list...")
    response = requests.get(f"{API_BASE_URL}/projects", headers=headers)
    print_json_response(response)
    
    try:
        projects_data = response.json()
        if projects_data.get("projects"):
            return projects_data["projects"][0]["id"]
    except (json.JSONDecodeError, KeyError, IndexError):
        pass
    
    return None

def test_storage_endpoints(headers, project_id):
    """Test 4: Storage endpoints."""
    print(f"4. Testing storage endpoints with project: {project_id}")
    
    # Test 4a: List storage objects
    print("4a. Listing storage objects...")
    response = requests.get(f"{API_BASE_URL}/projects/{project_id}/storage", headers=headers)
    print_json_response(response)
    print()
    
    # Test 4b: Get presigned URL for a sample file
    print("4b. Getting presigned URL for 'test.jpg'...")
    headers_with_expires = headers.copy()
    headers_with_expires["x-expires-in"] = "3600"
    
    response = requests.get(
        f"{API_BASE_URL}/projects/{project_id}/storage/test.jpg/url",
        headers=headers_with_expires
    )
    
    try:
        data = response.json()
        if "download_url" in data:
            print("✓ Presigned URL generated successfully")
        else:
            print("⚠️  File not found (expected for non-existent file)")
        print_json_response(response)
    except json.JSONDecodeError:
        print("⚠️  Failed to parse response")
        print(response.text)
    print()
    
    # Test 4c: List tasks and get URLs for their resources
    print("4c. Listing tasks...")
    response = requests.get(f"{API_BASE_URL}/projects/{project_id}/tasks", headers=headers)
    print_json_response(response)
    print()
    
    # Test 4d: Extract task resource URLs and test resolved URLs
    print("4d. Testing resolved download URLs for task resources...")
    try:
        tasks_data = response.json()
        tasks = tasks_data.get("tasks", [])
        
        # Get up to 5 tasks with resource URLs
        tasks_with_resources = []
        for task in tasks[:5]:
            if isinstance(task, dict):
                # Handle both old format (task.resource_url) and new format (task.task.resource_url)
                resource_url = task.get("resource_url") or (task.get("task", {}).get("resource_url") if "task" in task else None)
                resolved_url = task.get("resolved_resource_url")
                
                if resource_url and resource_url != "null":
                    tasks_with_resources.append({
                        "name": task.get("name") or (task.get("task", {}).get("name") if "task" in task else "Unknown"),
                        "resource_url": resource_url,
                        "resolved_resource_url": resolved_url
                    })
        
        if tasks_with_resources:
            for task_info in tasks_with_resources:
                print(f"Task: {task_info['name']}")
                print(f"  Original URL: {task_info['resource_url']}")
                
                if task_info['resolved_resource_url']:
                    print(f"  ✓ Resolved URL: {task_info['resolved_resource_url']}")
                    
                    # Test if the resolved URL is accessible
                    print("  Testing download accessibility...")
                    try:
                        test_response = requests.get(task_info['resolved_resource_url'], timeout=5)
                        http_status = test_response.status_code
                    except requests.RequestException as e:
                        http_status = 0
                        print(f"    Request error: {e}")
                    
                    if http_status == 200:
                        print(f"  ✓ File is accessible (HTTP {http_status})")
                    else:
                        print(f"  ⚠️  File not accessible (HTTP {http_status})")
                else:
                    print("  ⚠️  No resolved URL provided")
                
                print()
        else:
            print("⚠️  No tasks with resource URLs found")
    except json.JSONDecodeError:
        print("⚠️  Failed to parse tasks response")
    
    print()

def main():
    # Load JWT token
    jwt_token = load_jwt_token()
    print(f"✓ JWT token loaded from {TOKEN_FILE}")
    
    headers = {
        "Authorization": f"Bearer {jwt_token}"
    }
    
    print("=== fast-tag API Testing ===")
    print(f"API URL: {API_BASE_URL}")
    print()
    
    # Run tests
    test_health_check()
    test_user_info(headers)
    project_id = test_projects_list(headers)
    
    if project_id:
        print()
        test_storage_endpoints(headers, project_id)
    else:
        print()
        print("⚠️  No projects found. Create a project first to test storage endpoints.")
    
    print()
    print("=== API Testing Complete ===")

if __name__ == "__main__":
    main()