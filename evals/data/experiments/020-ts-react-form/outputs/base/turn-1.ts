interface ShippingOption {
  id: string;
  name: string;
  price: number;
  estimated_days: number;
  carrier: string;
}

// Updated usage example within the component for context:
/*
<select style={styles.input} onChange={e => setFormData({...formData, shippingMethod: e.target.value})}>
  {shipping.map(s => (
    <option key={s.id} value={s.id}>
      {s.carrier} - {s.name} ({s.estimated_days} days) - ${s.price.toFixed(2)}
    </option>
  ))}
</select>
*/