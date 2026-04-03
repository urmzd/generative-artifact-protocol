@dataclass
class Milestone(BaseModel):
    name: str
    target_date: datetime
    status: Status
    project_id: UUID
    task_ids: List[UUID] = field(default_factory=list)

    @classmethod
    def create(cls, name: str, target_date: datetime, project_id: UUID) -> "Milestone":
        """Factory method to initialize a new milestone."""
        if target_date < datetime.utcnow():
            raise ValueError("Target date must be in the future")
        return cls(
            name=name,
            target_date=target_date,
            status=Status.TODO,
            project_id=project_id
        )

    def add_task(self, task_id: UUID) -> None:
        """Adds a task ID to the milestone and updates the timestamp."""
        if task_id not in self.task_ids:
            self.task_ids.append(task_id)
            self.touch()