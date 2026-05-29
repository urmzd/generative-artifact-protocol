import React, { useMemo, useState } from 'react';

export interface CartItem {
  id: string;
  name: string;
  quantity: number;
  price: number;
}

export interface ShippingOption {
  id: string;
  name: string;
  price: number;
  estimated_days: string;
  carrier: string;
}

export interface FormData {
  fullName: string;
  email: string;
  address1: string;
  address2: string;
  city: string;
  state: string;
  postalCode: string;
  country: string;
  shippingOption: string;
  cardNumber: string;
  cardExpiry: string;
  cardCvc: string;
  promoCode: string;
}

export interface ValidationErrors {
  [key: string]: string;
}

interface CheckoutFormProps {
  cartItems: CartItem[];
  shippingOptions: ShippingOption[];
  onSubmit?: (data: FormData) => Promise<void> | void;
  taxRate?: number;
  initialValues?: Partial<FormData>;
}

const defaultFormData: FormData = {
  fullName: '',
  email: '',
  address1: '',
  address2: '',
  city: '',
  state: '',
  postalCode: '',
  country: '',
  shippingOption: '',
  cardNumber: '',
  cardExpiry: '',
  cardCvc: '',
  promoCode: '',
};

const promoDiscounts: Record<string, number> = {
  SAVE10: 0.1,
  WELCOME15: 0.15,
  FREESHIP: 0.05,
};

function required(value: string): boolean {
  return value.trim().length > 0;
}

function validateEmail(email: string): boolean {
  const re = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
  return re.test(email.trim());
}

function validateCardNumber(cardNumber: string): boolean {
  const digits = cardNumber.replace(/\D/g, '');
  if (digits.length < 13 || digits.length > 19) return false;

  let sum = 0;
  let shouldDouble = false;

  for (let i = digits.length - 1; i >= 0; i--) {
    let digit = parseInt(digits[i], 10);
    if (shouldDouble) {
      digit *= 2;
      if (digit > 9) digit -= 9;
    }
    sum += digit;
    shouldDouble = !shouldDouble;
  }

  return sum % 10 === 0;
}

function validateExpiryDate(expiry: string): boolean {
  const match = expiry.trim().match(/^(\d{2})\/(\d{2})$/);
  if (!match) return false;

  const month = parseInt(match[1], 10);
  const year = 2000 + parseInt(match[2], 10);

  if (month < 1 || month > 12) return false;

  const now = new Date();
  const currentMonth = now.getMonth() + 1;
  const currentYear = now.getFullYear();

  if (year < currentYear) return false;
  if (year === currentYear && month < currentMonth) return false;

  return true;
}

function formatCardNumber(value: string): string {
  const digits = value.replace(/\D/g, '').slice(0, 19);
  return digits.replace(/(.{4})/g, '$1 ').trim();
}

function formatExpiry(value: string): string {
  const digits = value.replace(/\D/g, '').slice(0, 4);
  if (digits.length <= 2) return digits;
  return `${digits.slice(0, 2)}/${digits.slice(2)}`;
}

function validatePromoCode(code: string): { valid: boolean; discountRate: number; message?: string } {
  const normalized = code.trim().toUpperCase();
  if (!normalized) return { valid: false, discountRate: 0, message: 'Please enter a promo code.' };
  const discountRate = promoDiscounts[normalized];
  if (!discountRate) return { valid: false, discountRate: 0, message: 'Invalid promo code.' };
  return { valid: true, discountRate, message: 'Promo code applied.' };
}

function validateFormData(data: FormData): ValidationErrors {
  const errors: ValidationErrors = {};

  if (!required(data.fullName)) errors.fullName = 'Full name is required.';
  if (!required(data.email)) {
    errors.email = 'Email is required.';
  } else if (!validateEmail(data.email)) {
    errors.email = 'Please enter a valid email address.';
  }

  if (!required(data.address1)) errors.address1 = 'Address is required.';
  if (!required(data.city)) errors.city = 'City is required.';
  if (!required(data.state)) errors.state = 'State is required.';
  if (!required(data.postalCode)) errors.postalCode = 'Postal code is required.';
  if (!required(data.country)) errors.country = 'Country is required.';
  if (!required(data.shippingOption)) errors.shippingOption = 'Please select a shipping option.';

  if (!required(data.cardNumber)) {
    errors.cardNumber = 'Card number is required.';
  } else if (!validateCardNumber(data.cardNumber)) {
    errors.cardNumber = 'Please enter a valid card number.';
  }

  if (!required(data.cardExpiry)) {
    errors.cardExpiry = 'Expiry date is required.';
  } else if (!validateExpiryDate(data.cardExpiry)) {
    errors.cardExpiry = 'Please enter a valid expiry date.';
  }

  if (!required(data.cardCvc)) {
    errors.cardCvc = 'CVC is required.';
  } else if (!/^\d{3,4}$/.test(data.cardCvc.trim())) {
    errors.cardCvc = 'Please enter a valid CVC.';
  }

  return errors;
}

type TextInputProps = {
  label: string;
  name: keyof FormData;
  value: string;
  error?: string;
  placeholder?: string;
  type?: string;
  onChange: (name: keyof FormData, value: string) => void;
};

function TextInput({ label, name, value, error, placeholder, type = 'text', onChange }: TextInputProps) {
  return (
    <div style={styles.field}>
      <label htmlFor={name} style={styles.label}>
        {label}
      </label>
      <input
        id={name}
        name={name}
        type={type}
        value={value}
        placeholder={placeholder}
        onChange={(e) => onChange(name, e.target.value)}
        style={{ ...styles.input, ...(error ? styles.inputError : {}) }}
      />
      {error ? <div style={styles.errorText}>{error}</div> : null}
    </div>
  );
}

type SelectInputProps = {
  label: string;
  name: keyof FormData;
  value: string;
  error?: string;
  options: { label: string; value: string }[];
  onChange: (name: keyof FormData, value: string) => void;
};

function SelectInput({ label, name, value, error, options, onChange }: SelectInputProps) {
  return (
    <div style={styles.field}>
      <label htmlFor={name} style={styles.label}>
        {label}
      </label>
      <select
        id={name}
        name={name}
        value={value}
        onChange={(e) => onChange(name, e.target.value)}
        style={{ ...styles.input, ...(error ? styles.inputError : {}) }}
      >
        <option value="">Select {label.toLowerCase()}</option>
        {options.map((opt) => (
          <option key={opt.value} value={opt.value}>
            {opt.label}
          </option>
        ))}
      </select>
      {error ? <div style={styles.errorText}>{error}</div> : null}
    </div>
  );
}

type CardNumberInputProps = {
  label: string;
  name: keyof FormData;
  value: string;
  error?: string;
  placeholder?: string;
  onChange: (name: keyof FormData, value: string) => void;
};

function CardNumberInput({ label, name, value, error, placeholder, onChange }: CardNumberInputProps) {
  return (
    <div style={styles.field}>
      <label htmlFor={name} style={styles.label}>
        {label}
      </label>
      <input
        id={name}
        name={name}
        inputMode="numeric"
        autoComplete="cc-number"
        value={value}
        placeholder={placeholder}
        onChange={(e) => onChange(name, formatCardNumber(e.target.value))}
        style={{ ...styles.input, ...(error ? styles.inputError : {}) }}
      />
      {error ? <div style={styles.errorText}>{error}</div> : null}
    </div>
  );
}

type ExpiryPickerProps = {
  label: string;
  name: keyof FormData;
  value: string;
  error?: string;
  onChange: (name: keyof FormData, value: string) => void;
};

function ExpiryPicker({ label, name, value, error, onChange }: ExpiryPickerProps) {
  return (
    <div style={styles.field}>
      <label htmlFor={name} style={styles.label}>
        {label}
      </label>
      <input
        id={name}
        name={name}
        inputMode="numeric"
        autoComplete="cc-exp"
        value={value}
        placeholder="MM/YY"
        onChange={(e) => onChange(name, formatExpiry(e.target.value))}
        style={{ ...styles.input, ...(error ? styles.inputError : {}) }}
      />
      {error ? <div style={styles.errorText}>{error}</div> : null}
    </div>
  );
}

export function CheckoutForm({
  cartItems,
  shippingOptions,
  onSubmit,
  taxRate = 0.08,
  initialValues,
}: CheckoutFormProps) {
  const [formData, setFormData] = useState<FormData>({ ...defaultFormData, ...initialValues });
  const [errors, setErrors] = useState<ValidationErrors>({});
  const [submitError, setSubmitError] = useState<string>('');
  const [promoMessage, setPromoMessage] = useState<string>('');
  const [discountRate, setDiscountRate] = useState<number>(0);
  const [isLoading, setIsLoading] = useState(false);

  const subtotal = useMemo(
    () => cartItems.reduce((sum, item) => sum + item.price * item.quantity, 0),
    [cartItems]
  );

  const selectedShipping = shippingOptions.find((option) => option.id === formData.shippingOption);
  const shippingCost = selectedShipping?.price ?? 0;
  const discount = subtotal * discountRate;
  const taxableAmount = subtotal - discount + shippingCost;
  const tax = taxableAmount * taxRate;
  const total = taxableAmount + tax;

  const handleChange = (name: keyof FormData, value: string) => {
    setFormData((prev) => ({ ...prev, [name]: value }));
    setErrors((prev) => ({ ...prev, [name]: '' }));
    setSubmitError('');
    if (name !== 'promoCode') setPromoMessage('');
  };

  const handleApplyPromo = () => {
    const result = validatePromoCode(formData.promoCode);
    if (result.valid) {
      setDiscountRate(result.discountRate);
      setPromoMessage(result.message || 'Promo code applied.');
      setErrors((prev) => ({ ...prev, promoCode: '' }));
    } else {
      setDiscountRate(0);
      setPromoMessage('');
      setErrors((prev) => ({ ...prev, promoCode: result.message || 'Invalid promo code.' }));
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const nextErrors = validateFormData(formData);
    setErrors(nextErrors);

    if (Object.keys(nextErrors).length > 0) return;

    setIsLoading(true);
    setSubmitError('');

    try {
      if (onSubmit) {
        await onSubmit(formData);
      } else {
        await new Promise((resolve) => setTimeout(resolve, 1000));
      }
    } catch (err) {
      setSubmitError(err instanceof Error ? err.message : 'Something went wrong submitting your order.');
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div style={styles.container}>
      <form onSubmit={handleSubmit} style={styles.form}>
        <div style={styles.main}>
          <div style={styles.section}>
            <h2 style={styles.sectionTitle}>Shipping Information</h2>
            <TextInput label="Full Name" name="fullName" value={formData.fullName} error={errors.fullName} onChange={handleChange} />
            <TextInput label="Email" name="email" type="email" value={formData.email} error={errors.email} onChange={handleChange} />
            <TextInput label="Address Line 1" name="address1" value={formData.address1} error={errors.address1} onChange={handleChange} />
            <TextInput label="Address Line 2" name="address2" value={formData.address2} error={errors.address2} onChange={handleChange} />
            <div style={styles.row}>
              <div style={styles.col}>
                <TextInput label="City" name="city" value={formData.city} error={errors.city} onChange={handleChange} />
              </div>
              <div style={styles.col}>
                <TextInput label="State" name="state" value={formData.state} error={errors.state} onChange={handleChange} />
              </div>
            </div>
            <div style={styles.row}>
              <div style={styles.col}>
                <TextInput label="Postal Code" name="postalCode" value={formData.postalCode} error={errors.postalCode} onChange={handleChange} />
              </div>
              <div style={styles.col}>
                <TextInput label="Country" name="country" value={formData.country} error={errors.country} onChange={handleChange} />
              </div>
            </div>
            <SelectInput
              label="Shipping Option"
              name="shippingOption"
              value={formData.shippingOption}
              error={errors.shippingOption}
              options={shippingOptions.map((opt) => ({
                label: `${opt.name} — ${opt.carrier} (${opt.estimated_days}) - $${opt.price.toFixed(2)}`,
                value: opt.id,
              }))}
              onChange={handleChange}
            />
          </div>

          <div style={styles.section}>
            <h2 style={styles.sectionTitle}>Payment Information</h2>
            <CardNumberInput
              label="Card Number"
              name="cardNumber"
              value={formData.cardNumber}
              error={errors.cardNumber}
              placeholder="1234 5678 9012 3456"
              onChange={handleChange}
            />
            <div style={styles.row}>
              <div style={styles.col}>
                <ExpiryPicker label="Expiry Date" name="cardExpiry" value={formData.cardExpiry} error={errors.cardExpiry} onChange={handleChange} />
              </div>
              <div style={styles.col}>
                <TextInput label="CVC" name="cardCvc" value={formData.cardCvc} error={errors.cardCvc} onChange={handleChange} placeholder="123" />
              </div>
            </div>
          </div>

          <div style={styles.section}>
            <h2 style={styles.sectionTitle}>Promo Code</h2>
            <div style={styles.promoRow}>
              <div style={styles.promoInputWrap}>
                <TextInput
                  label="Promo Code"
                  name="promoCode"
                  value={formData.promoCode}
                  error={errors.promoCode}
                  placeholder="Enter promo code"
                  onChange={handleChange}
                />
              </div>
              <button type="button" onClick={handleApplyPromo} style={styles.applyButton}>
                Apply
              </button>
            </div>
            {promoMessage ? <div style={styles.promoMessage}>{promoMessage}</div> : null}
          </div>

          {submitError ? <div style={styles.submitError}>{submitError}</div> : null}

          <button type="submit" disabled={isLoading} style={{ ...styles.submitButton, ...(isLoading ? styles.submitButtonDisabled : {}) }}>
            {isLoading ? 'Processing...' : 'Place Order'}
          </button>
        </div>

        <aside style={styles.sidebar}>
          <div style={styles.summaryCard}>
            <h2 style={styles.sectionTitle}>Order Summary</h2>
            <div style={styles.summaryItems}>
              {cartItems.map((item) => (
                <div key={item.id} style={styles.summaryItem}>
                  <div>
                    <div style={styles.itemName}>{item.name}</div>
                    <div style={styles.itemMeta}>Qty {item.quantity}</div>
                  </div>
                  <div style={styles.itemPrice}>${(item.price * item.quantity).toFixed(2)}</div>
                </div>
              ))}
            </div>

            <div style={styles.summaryDivider} />

            <div style={styles.summaryRow}>
              <span>Subtotal</span>
              <span>${subtotal.toFixed(2)}</span>
            </div>
            {discountRate > 0 ? (
              <div style={styles.summaryRow}>
                <span>Discount</span>
                <span>- ${discount.toFixed(2)}</span>
              </div>
            ) : null}
            <div style={styles.summaryRow}>
              <span>Shipping</span>
              <span>${shippingCost.toFixed(2)}</span>
            </div>
            <div style={styles.summaryRow}>
              <span>Tax</span>
              <span>${tax.toFixed(2)}</span>
            </div>

            <div style={{ ...styles.summaryRow, ...styles.totalRow }}>
              <span>Total</span>
              <span>${total.toFixed(2)}</span>
            </div>
          </div>
        </aside>
      </form>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: {
    display: 'flex',
    justifyContent: 'center',
    padding: '24px',
    backgroundColor: '#f6f7fb',
    minHeight: '100vh',
    boxSizing: 'border-box',
  },
  form: {
    display: 'grid',
    gridTemplateColumns: 'minmax(0, 2fr) minmax(280px, 1fr)',
    gap: '24px',
    width: '100%',
    maxWidth: '1200px',
    alignItems: 'start',
  },
  main: {
    display: 'flex',
    flexDirection: 'column',
    gap: '16px',
  },
  sidebar: {
    position: 'sticky',
    top: '24px',
  },
  section: {
    backgroundColor: '#ffffff',
    borderRadius: '12px',
    padding: '20px',
    boxShadow: '0 1px 2px rgba(0,0,0,0.06)',
    border: '1px solid #e7eaf0',
  },
  summaryCard: {
    backgroundColor: '#ffffff',
    borderRadius: '12px',
    padding: '20px',
    boxShadow: '0 1px 2px rgba(0,0,0,0.06)',
    border: '1px solid #e7eaf0',
  },
  sectionTitle: {
    margin: '0 0 16px',
    fontSize: '18px',
    color: '#1f2937',
  },
  field: {
    marginBottom: '14px',
    display: 'flex',
    flexDirection: 'column',
  },
  label: {
    marginBottom: '6px',
    fontSize: '14px',
    color: '#374151',
    fontWeight: 600,
  },
  input: {
    height: '44px',
    border: '1px solid #d1d5db',
    borderRadius: '8px',
    padding: '0 12px',
    fontSize: '14px',
    outline: 'none',
    backgroundColor: '#fff',
  },
  inputError: {
    borderColor: '#ef4444',
    boxShadow: '0 0 0 3px rgba(239,68,68,0.12)',
  },
  errorText: {
    marginTop: '6px',
    color: '#ef4444',
    fontSize: '12px',
  },
  row: {
    display: 'flex',
    gap: '12px',
  },
  col: {
    flex: 1,
  },
  promoRow: {
    display: 'flex',
    gap: '12px',
    alignItems: 'flex-end',
  },
  promoInputWrap: {
    flex: 1,
  },
  applyButton: {
    height: '44px',
    border: '1px solid #d1d5db',
    borderRadius: '8px',
    padding: '0 16px',
    backgroundColor: '#f9fafb',
    color: '#111827',
    fontSize: '14px',
    fontWeight: 600,
    cursor: 'pointer',
  },
  promoMessage: {
    marginTop: '8px',
    color: '#047857',
    fontSize: '13px',
  },
  summaryItems: {
    display: 'flex',
    flexDirection: 'column',
    gap: '12px',
  },
  summaryItem: {
    display: 'flex',
    justifyContent: 'space-between',
    gap: '12px',
  },
  itemName: {
    fontSize: '14px',
    color: '#111827',
    fontWeight: 600,
  },
  itemMeta: {
    fontSize: '12px',
    color: '#6b7280',
    marginTop: '2px',
  },
  itemPrice: {
    fontSize: '14px',
    color: '#111827',
    whiteSpace: 'nowrap',
  },
  summaryDivider: {
    height: '1px',
    backgroundColor: '#e5e7eb',
    margin: '16px 0',
  },
  summaryRow: {
    display: 'flex',
    justifyContent: 'space-between',
    marginBottom: '10px',
    fontSize: '14px',
    color: '#374151',
  },
  totalRow: {
    marginTop: '12px',
    paddingTop: '12px',
    borderTop: '1px solid #e5e7eb',
    fontSize: '16px',
    fontWeight: 700,
    color: '#111827',
  },
  submitButton: {
    marginTop: '4px',
    height: '48px',
    border: 'none',
    borderRadius: '10px',
    backgroundColor: '#2563eb',
    color: '#fff',
    fontSize: '15px',
    fontWeight: 700,
    cursor: 'pointer',
  },
  submitButtonDisabled: {
    opacity: 0.7,
    cursor: 'not-allowed',
  },
  submitError: {
    backgroundColor: '#fef2f2',
    color: '#b91c1c',
    border: '1px solid #fecaca',
    padding: '12px',
    borderRadius: '10px',
    fontSize: '14px',
  },
};