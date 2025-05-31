# Fast Tag API - Social Login

## Setup

1. Copy environment file:
```bash
cp api/.env.example api/.env
```

2. Configure OAuth providers in `api/.env`:

### Google OAuth Setup
1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Create a new project or select existing one
3. Enable Google+ API
4. Create OAuth 2.0 credentials
5. Add redirect URI: `http://localhost:8080/auth/google/callback`
6. Copy Client ID and Secret to `.env`

### GitHub OAuth Setup
1. Go to GitHub Settings > Developer settings > OAuth Apps
2. Create a new OAuth App
3. Set Authorization callback URL: `http://localhost:8080/auth/github/callback`
4. Copy Client ID and Secret to `.env`

## API Endpoints

- `GET /auth/google` - Initiate Google OAuth flow
- `GET /auth/google/callback` - Google OAuth callback
- `GET /auth/github` - Initiate GitHub OAuth flow
- `GET /auth/github/callback` - GitHub OAuth callback

## Usage

1. Start database:
```bash
docker compose up
```

2. Start API (migrations run automatically):
```bash
cargo run -p api
```

## OAuth Flow

1. Redirect user to `/auth/google` or `/auth/github`
2. User authorizes with provider
3. Provider redirects to callback URL
4. API returns JWT token and user info:

```json
{
  "token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "user": {
    "id": "uuid",
    "email": "user@example.com",
    "name": "User Name",
    "avatar_url": "https://...",
    "provider": "google",
    "provider_id": "123456789",
    "created_at": "2024-01-01T00:00:00Z",
    "updated_at": "2024-01-01T00:00:00Z"
  }
}
```