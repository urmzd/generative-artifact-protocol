<aap:target id="checkout-component">
interface CartItem {
  id: string;
  name: string;
  price: number;
}

interface ShippingOption {
  id: string;
  label: string;
  price: number;
}

interface FormData {
  email: string;
  cardNumber: string;
  expiry: string;
  cvv: string;
}

interface ValidationErrors {
  [key: string]: string;
}

const CheckoutForm: React.FC<{ items: CartItem[]; shippingOptions: ShippingOption[] }> = ({ items, shippingOptions }) => {
  <aap:target id="form-state">
  const [formData, setFormData] = React.useState<FormData>({ email: '', cardNumber: '', expiry: '', cvv: '' });
  const [errors, setErrors] = React.useState<ValidationErrors>({});
  const [loading, setLoading] = React.useState(false);
  </aap:target>

  const validate = (): boolean => {
    const newErrors: ValidationErrors = {};
    if (!/^\S+@\S+\.\S+$/.test(formData.email)) newErrors.email = "Invalid email";
    // Luhn check logic here
    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!validate()) return;
    setLoading(true);
    // Submit logic
    setLoading(false);
  };

  return (
    <div style={{ display: 'flex', gap: '2rem', padding: '2rem' }}>
      <form onSubmit={handleSubmit} style={{ flex: 2 }}>
        <aap:target id="form-fields">
          <input 
            type="email" 
            placeholder="Email" 
            onChange={(e) => setFormData({...formData, email: e.target.value})} 
          />
          {errors.email && <span style={{ color: 'red' }}>{errors.email}</span>}
        </aap:target>
        <button type="submit" disabled={loading}>
          <aap:target id="submit-button-text">Place Order</aap:target>
        </button>
      </form>

      <aside style={{ flex: 1, border: '1px solid #ccc', padding: '1rem' }}>
        <h3>Order Summary</h3>
        <aap:target id="order-summary-list">
          {items.map(item => <div key={item.id}>{item.name}: ${item.price}</div>)}
        </aap:target>
        <div style={{ marginTop: '1rem', borderTop: '1px solid #000' }}>
          <strong>Total: </strong>
          <aap:target id="total-price">$0.00</aap:target>
        </div>
      </aside>
    </div>
  );
};
</aap:target>