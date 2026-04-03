import argparse
from collections import defaultdict, Counter
from datetime import datetime
from typing import List, Dict, Any
import re
import json
import csv
import sys

<aap:target id="analysis-functions">
def parse_log_line(line: str) -> Dict[str, Any]:
    # Simplified regex for demo purposes
    pattern = r'(?P<ip>\S+) - - \[(?P<timestamp>.*?)\] "(?P<request>.*?)" (?P<status>\d+) (?P<size>\d+)'
    match = re.match(pattern, line)
    return match.groupdict() if match else None

def analyze_logs(logs: List[Dict[str, Any]], top_n: int = 10) -> Dict[str, Any]:
    ips = Counter(log['ip'] for log in logs)
    return {"top_ips": ips.most_common(top_n)}
</aap:target>

<aap:target id="table-formatter">
def format_output(data: Any, format_type: str) -> str:
    if format_type == 'json':
        return json.dumps(data, indent=2)
    elif format_type == 'csv':
        # Simple implementation
        output = []
        for ip, count in data['top_ips']:
            output.append(f"{ip},{count}")
        return "\n".join(output)
    else:
        # Box-drawing table
        lines = []
        lines.append("┌───────────────┬───────┐")
        lines.append("│ IP Address    │ Count │")
        lines.append("├───────────────┼───────┤")
        for ip, count in data['top_ips']:
            lines.append(f"│ {ip:<13} │ {count:<5} │")
        lines.append("└───────────────┴───────┘")
        return "\n".join(lines)
</aap:target>

<aap:target id="main-function">
def main() -> None:
    parser = argparse.ArgumentParser(description="CLI Log Analyzer")
    parser.add_argument("--file", required=True, help="Path to log file")
    parser.add_argument("--format", choices=['table', 'json', 'csv'], default='table')
    parser.add_argument("--top", type=int, default=10)
    args = parser.parse_args()
    
    with open(args.file, 'r') as f:
        logs = [parse_log_line(line) for line in f if parse_log_line(line)]
    
    results = analyze_logs(logs, top_n=args.top)
    print(format_output(results, args.format))
</aap:target>

if __name__ == "__main__":
    main()
