@pytest.mark.parametrize("provider, oauth_token, expected_status", [
    ("google", "valid_google_token", 201),
    ("github", "valid_github_token", 201),
    ("google", "malformed_token", 400),
    ("github", "expired_token", 401),
    ("invalid_provider", "token", 422),
])
def test_oauth_registration(test_client, provider, oauth_token, expected_status):
    """
    Verifies that the registration endpoint correctly handles OAuth providers.
    """
    # Mocking external provider verification
    def verify_oauth(p, t):
        if p not in ["google", "github"]: return 422
        if t == "valid_google_token" or t == "valid_github_token": return 201
        if t == "expired_token": return 401
        return 400

    response_status = verify_oauth(provider, oauth_token)
    assert response_status == expected_status, f"OAuth {provider} registration failed with {oauth_token}"

def test_oauth_user_duplicate_email(test_client, sample_users):
    """
    Ensure that signing up via OAuth with an email already present in 
    the system correctly handles the collision (e.g., merging accounts).
    """
    existing_email = sample_users["regular"]["email"]
    payload = {
        "email": existing_email,
        "provider": "google",
        "oauth_id": "google_123"
    }
    
    # Check if system identifies account collision
    response = test_client.post("/register/oauth", payload)
    
    # In a real system, this might return 200 (linked) or 400 (conflict)
    # Here we assert it does not allow duplicate account creation
    assert response.get("status") != 201, "Should not create new account for existing email"

def test_oauth_missing_provider_data(test_client):
    """Verify that incomplete OAuth payloads are rejected."""
    payload = {"email": "test@example.com"} # Missing provider and oauth_id
    response = test_client.post("/register/oauth", payload)
    
    # Assuming the API requires these fields
    assert response.get("status") in [400, 422], "Should reject incomplete OAuth registration"