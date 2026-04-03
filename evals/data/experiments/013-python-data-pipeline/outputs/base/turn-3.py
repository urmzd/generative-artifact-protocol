def load(df: pd.DataFrame, output_dir: str):
    path = Path(output_dir)
    path.mkdir(parents=True, exist_ok=True)
    
    # Parquet export
    df.to_parquet(path / "sales_processed.parquet")
    
    # Regional Summary CSV
    # Aggregating total revenue, order count (rows), and average order value
    regional_summary = df.groupby('region').agg(
        total_revenue=('revenue', 'sum'),
        order_count=('revenue', 'count'),
        avg_order_value=('revenue', 'mean')
    ).reset_index()
    
    regional_summary.to_csv(path / "regional_summary.csv", index=False)
    
    # JSON Summary (General metrics)
    summary = {
        "total_revenue": float(df['revenue'].sum()),
        "avg_margin": float(df['profit_margin'].mean()),
        "total_orders": int(len(df))
    }
    with open(path / "summary.json", 'w') as f:
        json.dump(summary, f, indent=4)