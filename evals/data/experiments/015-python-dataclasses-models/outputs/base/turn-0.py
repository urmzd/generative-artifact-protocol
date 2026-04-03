from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum
from typing import List, Optional, Type, TypeVar
from uuid import UUID, uuid4

T = TypeVar("T", bound="BaseModel")

class Priority(Enum):
    LOW = "low"
    MEDIUM = "medium"
    HIGH = "high"
    CRITICAL = "critical"

class Status(Enum):
    TODO = "todo"
    IN_PROGRESS = "in_progress"
    DONE = "done"
    ARCHIVED = "archived"

@dataclass(frozen=True)
class TimeEstimate:
    hours: int

    def __post_init__(self):
        if self.hours < 0:
            raise ValueError("Estimate cannot be negative")

@dataclass(frozen=True)
class DateRange:
    start: datetime
    end: datetime

    def __post_init__(self):
        if self.start > self.end:
            raise ValueError("Start date must be before end date")

@dataclass
class BaseModel:
    id: UUID = field(default_factory=uuid4)
    created_at: datetime = field(default_factory=datetime.utcnow)
    updated_at: datetime = field(default_factory=datetime.utcnow)

    def touch(self) -> None:
        self.updated_at = datetime.utcnow()

@dataclass
class User(BaseModel):
    username: str
    email: str

@dataclass
class Team(BaseModel):
    name: str
    member_ids: List[UUID] = field(default_factory=list)

@dataclass
class Project(BaseModel):
    name: str
    owner_id: UUID
    team_ids: List[UUID] = field(default_factory=list)

@dataclass
class Sprint(BaseModel):
    project_id: UUID
    name: str
    duration: DateRange

@dataclass
class Task(BaseModel):
    project_id: UUID
    title: str
    status: Status = Status.TODO
    priority: Priority = Priority.MEDIUM
    assignee_id: Optional[UUID] = None
    sprint_id: Optional[UUID] = None
    estimate: Optional[TimeEstimate] = None

    @classmethod
    def create(cls: Type[T], title: str, project_id: UUID, **kwargs) -> T:
        if not title:
            raise ValueError("Title is required")
        return cls(title=title, project_id=project_id, **kwargs)

@dataclass
class Comment(BaseModel):
    task_id: UUID
    author_id: UUID
    content: str

    def __post_init__(self):
        if not self.content.strip():
            raise ValueError("Comment content cannot be empty")