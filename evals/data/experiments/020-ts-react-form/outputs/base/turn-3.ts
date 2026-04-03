import React, { useState } from 'react';

interface CartItem {
  id: string;
  name: string;
  price: number;
}

interface ShippingOption {
  id: string;
  name: string;
  price: number;
  estimated_days: number;
  carrier: string;
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
  let sum = 0, shouldDouble = false;
  for (let i = digits.length - 1; i >= 0; i--) {
    let digit = digits[i];
    if (shouldDouble) {
      digit *= 2;
      if (digit > 9) digit -= 9;
    }
    sum += digit;
    shouldDouble = !shouldDouble;
  }
  return sum % 10 === 0 && digits.length > 0;
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
  const [promoInput, setPromoInput] = useState('');
  const [discount, setDiscount] = useState(0);

  const validateField = (name: keyof ValidationErrors, value: string) => {
    let error = '';
    if (name === 'email' && !validateEmail(value)) error = 'Invalid email address';
    if (name === 'cardNumber' && !validateLuhn(value)) error = 'Invalid card number';
    if (name === 'expiry' && !validateExpiry(value)) error = 'Invalid MM/YY';
    setErrors(prev => ({ ...prev, [name]: error }));
  };

  const subtotal = items.reduce((acc, item) => acc + item.price, 0);
  const ship = shipping.find(s => s.id === formData.shippingMethod);
  const total = Math.max(0, subtotal + (ship?.price || 0) - discount);

  return (
    <div style={styles.container}>
      <div style={styles.form}>
        <div style={styles.inputGroup}>
          <label>Email</label>
          <input style={styles.input} onBlur={(e) => validateField('email', e.target.value)} onChange={e => setFormData({...formData, email: e.target.value})} />
          {errors.email && <span style={styles.error}>{errors.email}</span>}
        </div>

        <div style={styles.inputGroup}>
          <label>Card Number</label>
          <input style={styles.input} onBlur={(e) => validateField('cardNumber', e.target.value)} onChange={e => setFormData({...formData, cardNumber: e.target.value})} />
          {errors.cardNumber && <span style={styles.error}>{errors.cardNumber}</span>}
        </div>

        <div style={styles.inputGroup}>
          <label>Expiry (MM/YY)</label>
          <input style={styles.input} onBlur={(e) => validateField('expiry', e.target.value)} onChange={e => setFormData({...formData, expiry: e.target.value})} />
          {errors.expiry && <span style={styles.error}>{errors.expiry}</span>}
        </div>
      </div>

      <div style={styles.sidebar}>
        <h3>Order Summary</h3>
        <div>Subtotal: ${subtotal.toFixed(2)}</div>
        {discount > 0 && <div>Discount: -${discount.toFixed(2)}</div>}
        <div>Shipping: ${ship?.price.toFixed(2)}</div>
        <strong>Total: ${total.toFixed(2)}</strong>
      </div>
    </div>
  );
};