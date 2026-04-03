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
  promoCode: string;
}

const styles: Record<string, React.CSSProperties> = {
  container: { display: 'flex', gap: '40px', padding: '20px', maxWidth: '900px', margin: '0 auto' },
  form: { flex: 2 },
  sidebar: { flex: 1, backgroundColor: '#f9f9f9', padding: '20px', borderRadius: '8px' },
  inputGroup: { marginBottom: '15px', display: 'flex', flexDirection: 'column' },
  inlineGroup: { display: 'flex', gap: '10px' },
  input: { padding: '8px', fontSize: '16px', borderRadius: '4px', border: '1px solid #ccc', flex: 1 },
  button: { padding: '8px 16px', cursor: 'pointer' }
};

export const CheckoutForm: React.FC<{ items: CartItem[], shipping: ShippingOption[] }> = ({ items, shipping }) => {
  const [formData, setFormData] = useState<FormData>({ email: '', cardNumber: '', expiry: '', shippingMethod: shipping[0].id, promoCode: '' });
  const [discount, setDiscount] = useState(0);
  const [promoInput, setPromoInput] = useState('');

  const applyPromo = () => {
    if (promoInput.toUpperCase() === 'SAVE10') setDiscount(10);
    else alert('Invalid Promo Code');
  };

  const subtotal = items.reduce((acc, item) => acc + item.price, 0);
  const selectedShipping = shipping.find(s => s.id === formData.shippingMethod);
  const shippingCost = selectedShipping?.price || 0;
  const total = Math.max(0, subtotal + shippingCost - discount);

  return (
    <div style={styles.container}>
      <form style={styles.form}>
        <div style={styles.inputGroup}>
          <label>Promo Code</label>
          <div style={styles.inlineGroup}>
            <input 
              style={styles.input} 
              value={promoInput} 
              onChange={(e) => setPromoInput(e.target.value)} 
              placeholder="Enter code" 
            />
            <button type="button" style={styles.button} onClick={applyPromo}>Apply</button>
          </div>
        </div>
      </form>

      <div style={styles.sidebar}>
        <h3>Summary</h3>
        <div>Subtotal: ${subtotal.toFixed(2)}</div>
        {discount > 0 && <div>Discount: -${discount.toFixed(2)}</div>}
        <div>Shipping: ${shippingCost.toFixed(2)}</div>
        <strong>Total: ${total.toFixed(2)}</strong>
      </div>
    </div>
  );
};