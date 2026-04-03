{
  "protocol": "aap/0.1",
  "id": "log-analyzer-script",
  "version": 2,
  "name": "edit",
  "content": [
    {
      "op": "insert_after",
      "target": {
        "type": "id",
        "value": "analysis-functions"
      },
      "content": "\ndef group_logs_by_time(logs: List[Dict[str, Any]], interval: str) -> Dict[str, List[Dict[str, Any]]}:\n    grouped = defaultdict(list)\n    for log in logs:\n        # Assuming timestamp format: [dd/MMM/yyyy:HH:mm:ss +0000]\n        dt = datetime.strptime(log['timestamp'].split(':')[0], '%d/%b/%Y')\n        if interval == 'hour':\n            key = dt.strftime('%Y-%m-%d %H')\n        elif interval == 'day':\n            key = dt.strftime('%Y-%m-%d')\n        elif interval == 'week':\n            key = dt.strftime('%Y-W%U')\n        else:\n            key = 'all'\n        grouped[key].append(log)\n    return dict(grouped)\n"
    },
    {
      "op": "replace",
      "target": {
        "type": "id",
        "value": "main-function"
      },
      "content": "def main() -> None:\n    parser = argparse.ArgumentParser(description=\"CLI Log Analyzer\")\n    parser.add_argument(\"--file\", required=True, help=\"Path to log file\")\n    parser.add_argument(\"--format\", choices=['table', 'json', 'csv'], default='table')\n    parser.add_argument(\"--top\", type=int, default=10)\n    parser.add_argument(\"--group-by\", choices=['hour', 'day', 'week'], help=\"Group results by time interval\")\n    \n    args = parser.parse_args()\n    \n    with open(args.file, 'r') as f:\n        logs = [parse_log_line(line) for line in f if parse_log_line(line)]\n    \n    if args.group_by:\n        grouped_logs = group_logs_by_time(logs, args.group_by)\n        results = {interval: analyze_logs(logs, top_n=args.top) for interval, logs in grouped_logs.items()}\n    else:\n        results = analyze_logs(logs, top_n=args.top)\n\n    print(format_output(results, args.format))"
    }
  ]
}