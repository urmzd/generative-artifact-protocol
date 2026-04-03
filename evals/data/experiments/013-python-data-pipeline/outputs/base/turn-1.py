def transform(df: pd.DataFrame) -> pd.DataFrame:
    # Existing cleaning and metrics
    df.columns = [c.lower().replace(' ', '_') for c in df.columns]
    df['profit'] = df['revenue'] - df['cost']
    df['profit_margin'] = df['profit'] / df['revenue']
    
    # New: Customer Lifetime Value (CLV)
    # Group by customer_id to calculate total spend across all history
    clv_map = df.groupby('customer_id')['revenue'].sum().rename('customer_lifetime_value')
    df = df.merge(clv_map, on='customer_id', how='left')
    
    # Categorization
    df['product_tier'] = pd.cut(
        df['revenue'], 
        bins=[0, 1000, 5000, np.inf], 
        labels=['Entry', 'Mid', 'Premium']
    )
    
    # YoY Growth
    df = df.sort_values(['region', 'date'])
    df['yoy_growth'] = df.groupby('region')['revenue'].pct_change(periods=1)
    
    return df