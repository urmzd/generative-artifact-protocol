from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum
from typing import List
from uuid import UUID

class Status(Enum):
    TODO = "TODO"
    IN_PROGRESS = "IN_PROGRESS"
    DONE = "DONE"

<aap:target id="priority-enum">
class Priority(Enum):
    LOW = "LOW"
    MEDIUM = "MEDIUM"
    HIGH = "HIGH"
    URGENT = "URGENT"

    @property
    def color(self) -> str:
        return {
            Priority.LOW: "#00FF00",
            Priority.MEDIUM: "#FFFF00",
            Priority.HIGH: "#FF8C00",
            Priority.URGENT: "#FF0000"
        }[self]
</aap:target>

@dataclass
class BaseEntity:
    id: UUID = None
    created_at: datetime = field(default_factory=datetime.now)

<aap:target id="project-models">
@dataclass
class Milestone(BaseEntity):
    name: str = ""
    target_date: datetime = None
    status: Status = Status.TODO
    project_id: UUID = None
    task_ids: List[UUID] = field(default_factory=list)
</aap:target>
