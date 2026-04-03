def validate(df: pd.DataFrame, config: PipelineConfig):
    # Check required columns
    missing = [c for c in config.required_cols if c not in df.columns]
    if missing: raise ValueError(f"Missing columns: {missing}")
    
    # Null checks
    if df[config.required_cols].isnull().any().any():
        print("Warning: Nulls detected in critical columns")
        
    # Range check: Sales cannot be negative
    if (df['sales'] < 0).any():
        raise ValueError("Negative sales detected")
        
    # Future date check
    today = pd.Timestamp.now()
    if (df['date'] > today).any():
        future_rows = df[df['date'] > today].shape[0]
        raise ValueError(f"Validation failed: {future_rows} rows have a future date.")
        
    # Duplicate check
    if df.duplicated().any():
        df.drop_duplicates(inplace=True)
        
    return df