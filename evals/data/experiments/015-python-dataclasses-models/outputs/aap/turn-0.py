from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum
from typing import List, Optional
from uuid import UUID, uuid4

<aap:target id="project-models">
class Priority(Enum):
    LOW = "low"
    MEDIUM = "medium"
    HIGH = "high"
    URGENT = "urgent"

class Status(Enum):
    TODO = "todo"
    IN_PROGRESS = "in_progress"
    REVIEW = "review"
    DONE = "done"

@dataclass(frozen=True)
class TimeEstimate:
    hours: int
    minutes: int

    def __post_init__(self):
        if self.hours < 0 or self.minutes < 0:
            raise ValueError("Time values cannot be negative")

@dataclass(frozen=True)
class DateRange:
    start: datetime
    end: datetime

    def __post_init__(self):
        if self.start > self.end:
            raise ValueError("Start date must be before end date")

@dataclass
class BaseEntity:
    <aap:target id="base-fields">
    id: UUID = field(default_factory=uuid4)
    created_at: datetime = field(default_factory=datetime.utcnow)
    updated_at: datetime = field(default_factory=datetime.utcnow)
    </aap:target>

@dataclass
class User(BaseEntity):
    name: str = "<aap:target id="user-name">Unknown User</aap:target>"
    email: str = ""

@dataclass
class Team(BaseEntity):
    name: str = ""
    member_ids: List[UUID] = field(default_factory=list)

@dataclass
class Project(BaseEntity):
    name: str = ""
    lead_id: Optional[UUID] = None

@dataclass
class Sprint(BaseEntity):
    project_id: UUID = None
    duration: DateRange = None

@dataclass
class Task(BaseEntity):
    project_id: UUID = None
    sprint_id: Optional[UUID] = None
    assignee_id: Optional[UUID] = None
    title: str = ""
    <aap:target id="task-metadata">
    priority: Priority = Priority.MEDIUM
    status: Status = Status.TODO
    estimate: Optional[TimeEstimate] = None
    </aap:target>

@dataclass
class Comment(BaseEntity):
    task_id: UUID = None
    author_id: UUID = None
    content: str = ""
</aap:target>