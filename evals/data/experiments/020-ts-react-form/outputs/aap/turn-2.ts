{
  "protocol": "aap/0.1",
  "id": "checkout-component",
  "version": 2,
  "name": "edit",
  "content": [
    {
      "op": "replace",
      "target": {
        "type": "id",
        "value": "checkout-component"
      },
      "content": "<aap:target id=\"checkout-component\">\ninterface CartItem {\n  id: string;\n  name: string;\n  price: number;\n}\n\ninterface ShippingOption {\n  id: string;\n  name: string;\n  price: number;\n  estimated_days: number;\n  carrier: string;\n}\n\ninterface FormData {\n  email: string;\n  cardNumber: string;\n  expiry: string;\n  cvv: string;\n}\n\ninterface ValidationErrors {\n  [key: string]: string;\n}\n\nconst CheckoutForm: React.FC<{ items: CartItem[]; shippingOptions: ShippingOption[] }> = ({ items, shippingOptions }) => {\n  <aap:target id=\"form-state\">\n  const [formData, setFormData] = React.useState<FormData>({ email: '', cardNumber: '', expiry: '', cvv: '' });\n  const [errors, setErrors] = React.useState<ValidationErrors>({});\n  const [loading, setLoading] = React.useState(false);\n  </aap:target>\n\n  const validate = (): boolean => {\n    const newErrors: ValidationErrors = {};\n    if (!/^\\S+@\\S+\\.\\S+$/.test(formData.email)) newErrors.email = \"Invalid email\";\n    // Luhn check logic here\n    setErrors(newErrors);\n    return Object.keys(newErrors).length === 0;\n  };\n\n  const handleSubmit = async (e: React.FormEvent) => {\n    e.preventDefault();\n    if (!validate()) return;\n    setLoading(true);\n    // Submit logic\n    setLoading(false);\n  };\n\n  return (\n    <div style={{ display: 'flex', gap: '2rem', padding: '2rem' }}>\n      <form onSubmit={handleSubmit} style={{ flex: 2 }}>\n        <aap:target id=\"form-fields\">\n          <input \n            type=\"email\" \n            placeholder=\"Email\" \n            onChange={(e) => setFormData({...formData, email: e.target.value})} \n          />\n          {errors.email && <span style={{ color: 'red' }}>{errors.email}</span>}\n        </aap:target>\n        <button type=\"submit\" disabled={loading}>\n          <aap:target id=\"submit-button-text\">Place Order</aap:target>\n        </button>\n      </form>\n\n      <aside style={{ flex: 1, border: '1px solid #ccc', padding: '1rem' }}>\n        <h3>Order Summary</h3>\n        <aap:target id=\"order-summary-list\">\n          {items.map(item => <div key={item.id}>{item.name}: ${item.price}</div>)}\n        </aap:target>\n        <div style={{ marginTop: '1rem', borderTop: '1px solid #000' }}>\n          <strong>Total: </strong>\n          <aap:target id=\"total-price\">$0.00</aap:target>\n        </div>\n      </aside>\n    </div>\n  );\n};\n</aap:target>"
    }
  ]
}