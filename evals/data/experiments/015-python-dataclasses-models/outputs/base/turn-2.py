class Priority(Enum):
    LOW = "low"
    MEDIUM = "medium"
    HIGH = "high"
    URGENT = "urgent"
    CRITICAL = "critical"

    @property
    def color(self) -> str:
        """Returns the hex color code associated with the priority level."""
        colors = {
            Priority.LOW: "#808080",      # Gray
            Priority.MEDIUM: "#FFA500",   # Orange
            Priority.HIGH: "#FF8C00",     # Dark Orange
            Priority.URGENT: "#FF4500",   # Orange Red
            Priority.CRITICAL: "#FF0000"  # Red
        }
        return colors[self]