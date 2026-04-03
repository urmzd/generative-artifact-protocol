import pytest
from flask import Flask
from yourapp import create_app, db
from yourapp.models import User

<aap:target id="fixtures">
@pytest.fixture(scope='module')
def test_client():
    app = create_app('testing')
    with app.test_client() as client:
        with app.app_context():
            db.create_all()
        yield client
        with app.app_context():
            db.drop_all()

@pytest.fixture(scope='module')
def sample_users():
    user1 = User(email='admin@example.com', password='StrongPass123!', role='admin', active=True)
    user2 = User(email='user@example.com', password='StrongPass123!', role='user', active=True)
    user3 = User(email='inactive@example.com', password='StrongPass123!', role='user', active=False)
    return user1, user2, user3

@pytest.fixture(scope='module')
def sample_superadmin():
    return User(email='super@example.com', password='StrongPass123!', role='superadmin', active=True)

@pytest.fixture(scope='module')
def auth_headers(sample_users):
    user1, user2, _ = sample_users
    headers = {}
    headers['Authorization'] = f"Bearer {user1.get_token()}"
    return headers

@pytest.fixture(scope='module')
def superadmin_headers(sample_superadmin):
    headers = {}
    headers['Authorization'] = f"Bearer {sample_superadmin.get_token()}"
    return headers
</aap:target>

def test_valid_signup(test_client):
    response = test_client.post('/register', json={
        'email': 'newuser@example.com',
        'password': 'NewStrongPass123!'
    })
    assert response.status_code == 201, "Should create a new user successfully."

def test_duplicate_email(test_client, sample_users):
    user1, _, _ = sample_users
    test_client.post('/register', json={
        'email': user1.email,
        'password': 'SomePassword123!'
    })
    response = test_client.post('/register', json={
        'email': user1.email,
        'password': 'AnotherPassword123!'
    })
    assert response.status_code == 400, "Should not allow duplicate email registration."

@pytest.mark.parametrize("email, password, expected_status", [
    ('invalidemail', 'ValidPass123!', 400),
    ('valid@example.com', 'weak', 400),
    ('', 'ValidPass123!', 400),
    ('valid@example.com', '', 400),
])
def test_registration_validation(test_client, email, password, expected_status):
    response = test_client.post('/register', json={
        'email': email,
        'password': password
    })
    assert response.status_code == expected_status, f"Expected status {expected_status} for email: {email}, password: {password}"

def test_valid_login(test_client, sample_users):
    user1, _, _ = sample_users
    response = test_client.post('/login', json={
        'email': user1.email,
        'password': 'StrongPass123!'
    })
    assert response.status_code == 200, "Should log in successfully with correct credentials."

def test_wrong_password(test_client, sample_users):
    user1, _, _ = sample_users
    response = test_client.post('/login', json={
        'email': user1.email,
        'password': 'WrongPassword!'
    })
    assert response.status_code == 401, "Should return unauthorized for incorrect password."

def test_inactive_user_login(test_client, sample_users):
    _, _, user3 = sample_users
    response = test_client.post('/login', json={
        'email': user3.email,
        'password': 'StrongPass123!'
    })
    assert response.status_code == 403, "Should return forbidden for inactive users."

def test_rate_limiting(test_client):
    for _ in range(6):  # Assuming limit is 5 attempts
        response = test_client.post('/login', json={
            'email': 'wrong@example.com',
            'password': 'WrongPassword!'
        })
    assert response.status_code == 429, "Should be rate limited after too many requests."

def test_token_refresh(test_client, sample_users):
    user1, _, _ = sample_users
    login_response = test_client.post('/login', json={
        'email': user1.email,
        'password': 'StrongPass123!'
    })
    token = login_response.json['token']

    response = test_client.post('/token/refresh', headers={'Authorization': f'Bearer {token}'})
    assert response.status_code == 200, "Should successfully refresh token."

def test_admin_only_endpoint(test_client, auth_headers):
    response = test_client.get('/admin-only', headers=auth_headers)
    assert response.status_code == 200, "Admins should have access to admin-only endpoints."

def test_superadmin_access_all(test_client, superadmin_headers):
    for endpoint in ['/admin-only', '/resource/1', '/super-secret']:
        response = test_client.get(endpoint, headers=superadmin_headers)
        assert response.status_code == 200, f"Superadmin should have access to {endpoint}."

def test_resource_ownership(test_client, auth_headers, sample_users):
    user1, user2, _ = sample_users
    
    # Assuming we've created resources here
    response = test_client.get(f'/resource/{user2.id}', headers=auth_headers)
    assert response.status_code == 403, "Users should not access resources owned by others."

def test_role_escalation_prevention(test_client, sample_users, auth_headers):
    _, user2, _ = sample_users
    response = test_client.post('/promote', json={'email': user2.email}, headers=auth_headers)
    assert response.status_code == 403, "Regular users should not be allowed to promote roles."

@pytest.mark.parametrize("endpoint", [
    '/admin-only',
    '/resource/1'
])
def test_access_control(test_client, endpoint, auth_headers):
    response = test_client.get(endpoint, headers=auth_headers)
    assert response.status_code == 200, f"User should have access to {endpoint}."
