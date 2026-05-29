package com.example.productmanagement.service;

import com.example.productmanagement.dto.PagedResponse;
import com.example.productmanagement.dto.ProductRequest;
import com.example.productmanagement.dto.ProductResponse;
import com.example.productmanagement.dto.ProductSearchCriteria;
import java.math.BigDecimal;
import java.util.List;
import java.util.Map;
import org.springframework.data.domain.Sort;

/**
 * Service interface for product management business logic.
 */
public interface ProductService {

    /**
     * Creates a new product.
     *
     * @param request the product request
     * @return created product response
     */
    ProductResponse createProduct(ProductRequest request);

    /**
     * Retrieves a product by id.
     *
     * @param id the product id
     * @return product response
     */
    ProductResponse getProductById(Long id);

    /**
     * Updates an existing product.
     *
     * @param id the product id
     * @param request the product request
     * @return updated product response
     */
    ProductResponse updateProduct(Long id, ProductRequest request);

    /**
     * Deletes a product by id.
     *
     * @param id the product id
     */
    void deleteProduct(Long id);

    /**
     * Searches products using criteria with pagination and sorting.
     *
     * @param criteria the search criteria
     * @param page the page number
     * @param size the page size
     * @param sortBy the sort field
     * @param sortDirection the sort direction
     * @return paged response of products
     */
    PagedResponse<ProductResponse> searchProducts(ProductSearchCriteria criteria, int page, int size, String sortBy, Sort.Direction sortDirection);

    /**
     * Retrieves all products.
     *
     * @return list of all product responses
     */
    List<ProductResponse> getAllProducts();

    /**
     * Exports all products as CSV.
     *
     * @return CSV content
     */
    String exportProductsAsCsv();

    /**
     * Bulk updates product prices in a single transaction.
     *
     * @param priceUpdates map of product id to new price
     * @return updated product responses
     */
    List<ProductResponse> bulkUpdatePrices(Map<Long, BigDecimal> priceUpdates);
}

package com.example.productmanagement.service.impl;

import com.example.productmanagement.dto.PagedResponse;
import com.example.productmanagement.dto.ProductRequest;
import com.example.productmanagement.dto.ProductResponse;
import com.example.productmanagement.dto.ProductSearchCriteria;
import com.example.productmanagement.entity.Product;
import com.example.productmanagement.exception.ResourceNotFoundException;
import com.example.productmanagement.repository.ProductRepository;
import com.example.productmanagement.service.ProductService;
import java.math.BigDecimal;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.stream.Collectors;
import org.springframework.cache.annotation.CacheEvict;
import org.springframework.cache.annotation.CachePut;
import org.springframework.cache.annotation.Cacheable;
import org.springframework.data.domain.Page;
import org.springframework.data.domain.PageRequest;
import org.springframework.data.domain.Sort;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;

/**
 * Default implementation of ProductService.
 */
@Service
@Transactional
public class ProductServiceImpl implements ProductService {

    private final ProductRepository productRepository;

    /**
     * Creates a new ProductServiceImpl.
     *
     * @param productRepository the product repository
     */
    public ProductServiceImpl(ProductRepository productRepository) {
        this.productRepository = productRepository;
    }

    @Override
    @CachePut(value = "products", key = "#result.id")
    public ProductResponse createProduct(ProductRequest request) {
        validateRequest(request);
        Product product = new Product();
        product.setName(request.getName());
        product.setDescription(request.getDescription());
        product.setCategory(request.getCategory());
        product.setPrice(request.getPrice());
        product.setSku(request.getSku());
        product.setActive(request.getActive() != null ? request.getActive() : Boolean.TRUE);
        return toResponse(productRepository.save(product));
    }

    @Override
    @Transactional(readOnly = true)
    @Cacheable(value = "products", key = "#id")
    public ProductResponse getProductById(Long id) {
        Product product = productRepository.findById(id)
                .orElseThrow(() -> new ResourceNotFoundException("Product not found with id: " + id));
        return toResponse(product);
    }

    @Override
    @CachePut(value = "products", key = "#id")
    public ProductResponse updateProduct(Long id, ProductRequest request) {
        validateRequest(request);
        Product product = productRepository.findById(id)
                .orElseThrow(() -> new ResourceNotFoundException("Product not found with id: " + id));
        product.setName(request.getName());
        product.setDescription(request.getDescription());
        product.setCategory(request.getCategory());
        product.setPrice(request.getPrice());
        product.setSku(request.getSku());
        product.setActive(request.getActive() != null ? request.getActive() : product.getActive());
        return toResponse(productRepository.save(product));
    }

    @Override
    @CacheEvict(value = "products", key = "#id")
    public void deleteProduct(Long id) {
        if (!productRepository.existsById(id)) {
            throw new ResourceNotFoundException("Product not found with id: " + id);
        }
        productRepository.deleteById(id);
    }

    @Override
    @Transactional(readOnly = true)
    public PagedResponse<ProductResponse> searchProducts(ProductSearchCriteria criteria, int page, int size, String sortBy, Sort.Direction sortDirection) {
        PageRequest pageRequest = PageRequest.of(page, size, Sort.by(sortDirection, sortBy));
        Page<Product> productPage = productRepository.search(
                criteria.getName(),
                criteria.getCategory(),
                criteria.getMinPrice(),
                criteria.getMaxPrice(),
                pageRequest);
        List<ProductResponse> responses = productPage.getContent().stream().map(this::toResponse).collect(Collectors.toList());
        return new PagedResponse<>(
                responses,
                productPage.getNumber(),
                productPage.getSize(),
                productPage.getTotalElements(),
                productPage.getTotalPages(),
                productPage.isLast());
    }

    @Override
    @Transactional(readOnly = true)
    public List<ProductResponse> getAllProducts() {
        return productRepository.findAll().stream().map(this::toResponse).collect(Collectors.toList());
    }

    @Override
    @Transactional(readOnly = true)
    public String exportProductsAsCsv() {
        List<Product> products = productRepository.findAll();
        StringBuilder csv = new StringBuilder();
        csv.append("id,name,description,category,price,sku,active,createdAt,updatedAt\n");
        for (Product product : products) {
            csv.append(escapeCsv(product.getId()))
               .append(',')
               .append(escapeCsv(product.getName()))
               .append(',')
               .append(escapeCsv(product.getDescription()))
               .append(',')
               .append(escapeCsv(product.getCategory()))
               .append(',')
               .append(escapeCsv(product.getPrice()))
               .append(',')
               .append(escapeCsv(product.getSku()))
               .append(',')
               .append(escapeCsv(product.getActive()))
               .append(',')
               .append(escapeCsv(product.getCreatedAt()))
               .append(',')
               .append(escapeCsv(product.getUpdatedAt()))
               .append('\n');
        }
        return csv.toString();
    }

    @Override
    @Transactional
    public List<ProductResponse> bulkUpdatePrices(Map<Long, BigDecimal> priceUpdates) {
        if (priceUpdates == null || priceUpdates.isEmpty()) {
            return List.of();
        }

        List<ProductResponse> updatedProducts = new ArrayList<>();

        for (Map.Entry<Long, BigDecimal> entry : priceUpdates.entrySet()) {
            Long productId = entry.getKey();
            BigDecimal newPrice = entry.getValue();

            if (productId == null) {
                throw new IllegalArgumentException("Product id must not be null");
            }
            if (newPrice == null) {
                throw new IllegalArgumentException("Price must not be null for product id: " + productId);
            }
            if (newPrice.compareTo(BigDecimal.ZERO) < 0) {
                throw new IllegalArgumentException("Price must be non-negative for product id: " + productId);
            }

            Product product = productRepository.findById(productId)
                    .orElseThrow(() -> new ResourceNotFoundException("Product not found with id: " + productId));
            product.setPrice(newPrice);

            updatedProducts.add(toResponse(product));
        }

        productRepository.saveAll(
                updatedProducts.stream()
                        .map(response -> {
                            Product product = productRepository.findById(response.getId())
                                    .orElseThrow(() -> new ResourceNotFoundException("Product not found with id: " + response.getId()));
                            product.setPrice(response.getPrice());
                            return product;
                        })
                        .toList()
        );

        return updatedProducts;
    }

    private void validateRequest(ProductRequest request) {
        if (request.getPrice() != null && request.getPrice().compareTo(BigDecimal.ZERO) < 0) {
            throw new IllegalArgumentException("Price must be non-negative");
        }
    }

    private ProductResponse toResponse(Product product) {
        return new ProductResponse(
                product.getId(),
                product.getName(),
                product.getDescription(),
                product.getCategory(),
                product.getPrice(),
                product.getSku(),
                product.getActive(),
                product.getCreatedAt(),
                product.getUpdatedAt());
    }

    private String escapeCsv(Object value) {
        if (value == null) {
            return "";
        }
        String text = String.valueOf(value);
        boolean needsQuotes = text.contains(",") || text.contains("\"") || text.contains("\n") || text.contains("\r");
        text = text.replace("\"", "\"\"");
        return needsQuotes ? "\"" + text + "\"" : text;
    }
}