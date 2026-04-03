{
  "protocol": "aap/0.1",
  "id": "project-models",
  "version": 2,
  "name": "edit",
  "content": [
    {
      "op": "insert_after",
      "target": {
        "type": "id",
        "value": "project-models"
      },
      "content": "@dataclass\nclass Milestone(BaseEntity):\n    name: str = \"\"\n    target_date: datetime = None\n    status: Status = Status.TODO\n    project_id: UUID = None\n    task_ids: List[UUID] = field(default_factory=list)\n"
    }
  ]
}