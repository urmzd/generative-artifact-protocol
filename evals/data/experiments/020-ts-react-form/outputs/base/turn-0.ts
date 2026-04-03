import React, { useState, useMemo } from 'react';

interface CartItem {
  id: string;
  name: string;
  price: number;
}

interface ShippingOption {
  id: string;
  label: string;
  cost: number;
}

interface FormData {
  email: string;
  cardNumber: string;
  expiry: string;
  shippingMethod: string;
}

interface ValidationErrors {
  email?: string;
  cardNumber?: string;
  expiry?: string;
}

const validateEmail = (email: string) => /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email);

const validateLuhn = (number: string) => {
  const digits = number.replace(/\s+/g, '').split('').map(Number);
  let sum = 0;
  let shouldDouble = false;
  for (let i = digits.length - 1; i >= 0; i--) {
    let digit = digits[i];
    if (shouldDouble) {
      digit *= 2;
      if (digit > 9) digit -= 9;
    }
    sum += digit;
    shouldDouble = !shouldDouble;
  }
  return sum % 10 === 0;
};

const validateExpiry = (expiry: string) => {
  const [mm, yy] = expiry.split('/').map(Number);
  if (!mm || !yy || mm < 1 || mm > 12) return false;
  const now = new Date();
  const currentMonth = now.getMonth() + 1;
  const currentYear = parseInt(now.getFullYear().toString().slice(-2));
  return yy > currentYear || (yy === currentYear && mm >= currentMonth);
};

const styles: Record<string, React.CSSProperties> = {
  container: { display: 'flex', gap: '40px', padding: '20px', maxWidth: '900px', margin: '0 auto' },
  form: { flex: 2 },
  sidebar: { flex: 1, backgroundColor: '#f9f9f9', padding: '20px', borderRadius: '8px' },
  inputGroup: { marginBottom: '15px', display: 'flex', flexDirection: 'column' },
  input: { padding: '8px', fontSize: '16px', borderRadius: '4px', border: '1px solid #ccc' },
  error: { color: 'red', fontSize: '12px', marginTop: '4px' }
};

export const CheckoutForm: React.FC<{ items: CartItem[], shipping: ShippingOption[] }> = ({ items, shipping }) => {
  const [formData, setFormData] = useState<FormData>({ email: '', cardNumber: '', expiry: '', shippingMethod: shipping[0].id });
  const [errors, setErrors] = useState<ValidationErrors>({});
  const [loading, setLoading] = useState(false);

  const subtotal = items.reduce((acc, item) => acc + item.price, 0);
  const shippingCost = shipping.find(s => s.id === formData.shippingMethod)?.cost || 0;
  const tax = subtotal * 0.08;
  const total = subtotal + shippingCost + tax;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const newErrors: ValidationErrors = {};
    if (!validateEmail(formData.email)) newErrors.email = 'Invalid email';
    if (!validateLuhn(formData.cardNumber)) newErrors.cardNumber = 'Invalid card number';
    if (!validateExpiry(formData.expiry)) newErrors.expiry = 'Invalid expiry';
    
    if (Object.keys(newErrors).length > 0) {
      setErrors(newErrors);
      return;
    }

    setLoading(true);
    await new Promise(resolve => setTimeout(resolve, 1500));
    setLoading(false);
    alert('Order placed successfully!');
  };

  const handleCardChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    let val = e.target.value.replace(/\D/g, '').slice(0, 16);
    val = val.replace(/(\d{4})(?=\d)/g, '$1 ');
    setFormData({ ...formData, cardNumber: val });
  };

  return (
    <div style={styles.container}>
      <form style={styles.form} onSubmit={handleSubmit}>
        <h2>Checkout</h2>
        <div style={styles.inputGroup}>
          <label>Email</label>
          <input style={styles.input} type="email" onChange={e => setFormData({...formData, email: e.target.value})} />
          {errors.email && <span style={styles.error}>{errors.email}</span>}
        </div>
        <div style={styles.inputGroup}>
          <label>Card Number</label>
          <input style={styles.input} value={formData.cardNumber} onChange={handleCardChange} placeholder="0000 0000 0000 0000" />
          {errors.cardNumber && <span style={styles.error}>{errors.cardNumber}</span>}
        </div>
        <div style={styles.inputGroup}>
          <label>Expiry (MM/YY)</label>
          <input style={styles.input} maxLength={5} onChange={e => setFormData({...formData, expiry: e.target.value})} placeholder="MM/YY" />
          {errors.expiry && <span style={styles.error}>{errors.expiry}</span>}
        </div>
        <div style={styles.inputGroup}>
          <label>Shipping</label>
          <select style={styles.input} onChange={e => setFormData({...formData, shippingMethod: e.target.value})}>
            {shipping.map(s => <option key={s.id} value={s.id}>{s.label} (+${s.cost})</option>)}
          </select>
        </div>
        <button type="submit" disabled={loading}>{loading ? 'Processing...' : 'Pay Now'}</button>
      </form>

      <div style={styles.sidebar}>
        <h3>Order Summary</h3>
        {items.map(item => <div key={item.id}>{item.name}: ${item.price.toFixed(2)}</div>)}
        <hr />
        <div>Subtotal: ${subtotal.toFixed(2)}</div>
        <div>Shipping: ${shippingCost.toFixed(2)}</div>
        <div>Tax: ${tax.toFixed(2)}</div>
        <strong>Total: ${total.toFixed(2)}</strong>
      </div>
    </div>
  );
};