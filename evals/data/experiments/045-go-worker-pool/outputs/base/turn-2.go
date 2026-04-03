package workerpool

import (
	"context"
	"fmt"
	"math/rand"
	"sync"
	"time"
)

type ImageJob struct {
	ID     int
	Width  int
	Height int
	Prio   Priority
}

func (j *ImageJob) Execute(ctx context.Context) (interface{}, error) {
	// Simulate image resizing computation
	time.Sleep(time.Duration(rand.Intn(100)) * time.Millisecond)
	return fmt.Sprintf("Image %d resized to %dx%d", j.ID, j.Width, j.Height), nil
}

func (j *ImageJob) Priority() Priority {
	return j.Prio
}

func ExampleImageResizing() {
	pool := New(5, 20)
	pool.Start(5)

	// Submit 20 resize jobs
	go func() {
		for i := 1; i <= 20; i++ {
			// Randomly assign priority
			prio := PriorityLow
			if i%5 == 0 {
				prio = PriorityHigh
			} else if i%3 == 0 {
				prio = PriorityMedium
			}

			pool.Submit(&ImageJob{
				ID:     i,
				Width:  800 + (i * 10),
				Height: 600 + (i * 10),
				Prio:   prio,
			})
		}
		
		// Wait a bit then shut down
		time.Sleep(1 * time.Second)
		pool.Stop()
	}()

	for res := range pool.Results() {
		if res.Err != nil {
			fmt.Printf("Job failed: %v\n", res.Err)
		} else {
			fmt.Printf("Success: %v\n", res.Value)
		}
	}
}

// Keep the previous Priority/Job/WorkerPool definitions here...
// The orchestrator logic ensures PriorityHigh (int 2) > PriorityMedium (1) > PriorityLow (0)
// by using heap.Less returning a > b.