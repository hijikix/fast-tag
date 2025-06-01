CREATE TABLE pending_auths (
    id SERIAL PRIMARY KEY,
    auth_key VARCHAR(255) UNIQUE NOT NULL,
    csrf_token VARCHAR(255) NOT NULL,
    jwt TEXT,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_pending_auths_auth_key ON pending_auths(auth_key);
CREATE INDEX idx_pending_auths_expires_at ON pending_auths(expires_at);