from datetime import datetime
from typing import Generator, List, Optional

from fastapi import APIRouter, Depends, FastAPI, HTTPException, Query, status
from pydantic import BaseModel, ConfigDict, EmailStr, Field
from sqlalchemy import Boolean, DateTime, Integer, String, create_engine, select
from sqlalchemy.exc import IntegrityError
from sqlalchemy.orm import DeclarativeBase, Mapped, Session, mapped_column, sessionmaker
from sqlalchemy.sql import func

DATABASE_URL = "sqlite:///./users.db"

engine = create_engine(
    DATABASE_URL,
    connect_args={"check_same_thread": False} if DATABASE_URL.startswith("sqlite") else {},
)
SessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)


class Base(DeclarativeBase):
    pass


class User(Base):
    __tablename__ = "users"

    id: Mapped[int] = mapped_column(Integer, primary_key=True, index=True)
    email: Mapped[str] = mapped_column(String(255), unique=True, index=True, nullable=False)
    name: Mapped[str] = mapped_column(String(255), nullable=False)
    hashed_password: Mapped[str] = mapped_column(String(255), nullable=False)
    role: Mapped[str] = mapped_column(String(50), nullable=False, default="user")
    is_active: Mapped[bool] = mapped_column(Boolean, nullable=False, default=True)
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), nullable=False, server_default=func.now()
    )


Base.metadata.create_all(bind=engine)


def get_db() -> Generator[Session, None, None]:
    db = SessionLocal()
    try:
        yield db
    finally:
        db.close()


class UserCreate(BaseModel):
    email: EmailStr
    name: str = Field(min_length=1, max_length=255)
    password: str = Field(min_length=8, max_length=255)
    role: str = Field(default="user", max_length=50)
    is_active: bool = True


class UserUpdate(BaseModel):
    email: Optional[EmailStr] = None
    name: Optional[str] = Field(default=None, min_length=1, max_length=255)
    password: Optional[str] = Field(default=None, min_length=8, max_length=255)
    role: Optional[str] = Field(default=None, max_length=50)
    is_active: Optional[bool] = None


class UserResponse(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    id: int
    email: EmailStr
    name: str
    role: str
    is_active: bool
    created_at: datetime


class UserList(BaseModel):
    items: List[UserResponse]
    total: int
    page: int
    page_size: int


def hash_password(password: str) -> str:
    return f"hashed_{password}"


def create_user(db: Session, user_in: UserCreate) -> User:
    user = User(
        email=user_in.email,
        name=user_in.name,
        hashed_password=hash_password(user_in.password),
        role=user_in.role,
        is_active=user_in.is_active,
    )
    db.add(user)
    try:
        db.commit()
        db.refresh(user)
        return user
    except IntegrityError:
        db.rollback()
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail="User with this email already exists",
        )


def get_user(db: Session, user_id: int) -> User:
    user = db.get(User, user_id)
    if not user:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="User not found",
        )
    return user


def list_users(db: Session, page: int = 1, page_size: int = 10) -> UserList:
    if page < 1 or page_size < 1:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail="page and page_size must be greater than 0",
        )

    total = db.scalar(select(func.count()).select_from(User)) or 0
    offset = (page - 1) * page_size
    stmt = select(User).order_by(User.id).offset(offset).limit(page_size)
    items = list(db.scalars(stmt).all())
    return UserList(
        items=[UserResponse.model_validate(item) for item in items],
        total=total,
        page=page,
        page_size=page_size,
    )


def update_user(db: Session, user_id: int, user_in: UserUpdate) -> User:
    user = get_user(db, user_id)

    if user_in.email is not None:
        user.email = user_in.email
    if user_in.name is not None:
        user.name = user_in.name
    if user_in.password is not None:
        user.hashed_password = hash_password(user_in.password)
    if user_in.role is not None:
        user.role = user_in.role
    if user_in.is_active is not None:
        user.is_active = user_in.is_active

    try:
        db.commit()
        db.refresh(user)
        return user
    except IntegrityError:
        db.rollback()
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail="User with this email already exists",
        )


def delete_user(db: Session, user_id: int) -> None:
    user = get_user(db, user_id)
    db.delete(user)
    db.commit()


router = APIRouter(prefix="/users", tags=["Users"])


@router.post("", response_model=UserResponse, status_code=status.HTTP_201_CREATED)
def api_create_user(user_in: UserCreate, db: Session = Depends(get_db)):
    return create_user(db, user_in)


@router.get("", response_model=UserList)
def api_list_users(
    page: int = Query(1, ge=1),
    page_size: int = Query(10, ge=1, le=100),
    db: Session = Depends(get_db),
):
    return list_users(db, page=page, page_size=page_size)


@router.get("/{user_id}", response_model=UserResponse)
def api_get_user(user_id: int, db: Session = Depends(get_db)):
    return get_user(db, user_id)


@router.put("/{user_id}", response_model=UserResponse)
def api_update_user(user_id: int, user_in: UserUpdate, db: Session = Depends(get_db)):
    return update_user(db, user_id, user_in)


@router.delete("/{user_id}", status_code=status.HTTP_204_NO_CONTENT)
def api_delete_user(user_id: int, db: Session = Depends(get_db)):
    delete_user(db, user_id)
    return None


app = FastAPI(title="User Management Module")
app.include_router(router)