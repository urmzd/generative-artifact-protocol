package com.example.productmanagement.repository;

import com.example.productmanagement.entity.Product;
import java.math.BigDecimal;
import java.util.List;
import java.util.Optional;
import org.springframework.data.domain.Page;
import org.springframework.data.domain.Pageable;
import org.springframework.data.jpa.repository.JpaRepository;
import org.springframework.data.jpa.repository.Query;
import org.springframework.stereotype.Repository;

/**
 * JPA repository for Product entity.
 */
@Repository
public interface ProductRepository extends JpaRepository<Product, Long> {

    /**
     * Finds a product by SKU.
     *
     * @param sku the SKU
     * @return optional product
     */
    Optional<Product> findBySku(String sku);

    /**
     * Finds products by category.
     *
     * @param category the category
     * @return list of products
     */
    List<Product> findByCategory(String category);

    /**
     * Finds products by active status.
     *
     * @param active active flag
     * @return list of products
     */
    List<Product> findByActive(Boolean active);

    /**
     * Searches products by optional criteria.
     *
     * @param name name filter
     * @param category category filter
     * @param minPrice min price filter
     * @param maxPrice max price filter
     * @param pageable pageable object
     * @return page of products
     */
    @Query("""
            SELECT p FROM Product p
            WHERE (:name IS NULL OR LOWER(p.name) LIKE LOWER(CONCAT('%', :name, '%')))
              AND (:category IS NULL OR LOWER(p.category) LIKE LOWER(CONCAT('%', :category, '%')))
              AND (:minPrice IS NULL OR p.price >= :minPrice)
              AND (:maxPrice IS NULL OR p.price <= :maxPrice)
            """)
    Page<Product> search(String name, String category, Double minPrice, Double maxPrice, Pageable pageable);

    /**
     * Finds products by price range and category using JPQL.
     *
     * @param minPrice minimum price
     * @param maxPrice maximum price
     * @param category category filter
     * @return matching products
     */
    @Query("""
            SELECT p
            FROM Product p
            WHERE p.price BETWEEN :minPrice AND :maxPrice
              AND (:category IS NULL OR LOWER(p.category) = LOWER(:category))
            """)
    List<Product> findByPriceRangeAndCategory(BigDecimal minPrice, BigDecimal maxPrice, String category);

    /**
     * Checks existence of a product by SKU.
     *
     * @param sku the SKU
     * @return true if exists
     */
    boolean existsBySku(String sku);
}